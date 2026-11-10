//! typing-pet 진입점: 이벤트 루프, 창 생성, 오프스크린 렌더 CLI.
//!
//! 흐름: 입력 이벤트 -> Pet(상태 머신) -> Animator(연출) -> Frame(그릴 목록) -> Renderer

use std::sync::Arc;
use std::time::{Duration, Instant};

use typing_pet::anim::Animator;
use typing_pet::config::{Config, Manifest};
use typing_pet::input::{self, Cursor, Input};
use typing_pet::pet::{Pet, PetState};
use typing_pet::render::{self, HitMap, Renderer};
use typing_pet::tray::{Tray, TrayAction};
use typing_pet::{mac, trace};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS, WindowAttributesExtMacOS};
use winit::window::{Window, WindowId, WindowLevel};

#[derive(Debug, Clone)]
enum AppEvent {
    Input(Input, Instant),
    Cursor(Cursor),
    Tray(TrayAction),
}

struct WindowGpu {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    hit: HitMap,
}

impl WindowGpu {
    fn new(window: Arc<Window>, m: &Manifest) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("어댑터 없음: {e}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| e.to_string())?;
        let caps = surface.get_capabilities(&adapter);
        log::info!("surface formats={:?} alpha_modes={:?} present_modes={:?}", caps.formats, caps.alpha_modes, caps.present_modes);
        // 감마 공간에서 섞기 위해 sRGB 가 아닌 포맷을 고른다
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let alpha_mode = [wgpu::CompositeAlphaMode::PreMultiplied, wgpu::CompositeAlphaMode::PostMultiplied]
            .into_iter()
            .find(|m| caps.alpha_modes.contains(m))
            .ok_or_else(|| format!("투명 alpha 모드 미지원: {:?}", caps.alpha_modes))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface 기본 설정 없음")?;
        config.format = format;
        config.alpha_mode = alpha_mode;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency =
            std::env::var("TYPING_PET_FRAME_LATENCY").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
        log::info!("surface format={format:?} alpha={alpha_mode:?}");
        surface.configure(&device, &config);
        let layers = render::load_layers(m);
        let hit = HitMap::new(&layers, 128);
        let mut renderer = Renderer::new(device, queue, format, layers, m.canvas);
        renderer.prepare(size.width.max(size.height));
        Ok(Self { surface, config, renderer, hit })
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.renderer.device, &self.config);
        self.renderer.prepare(w.max(h));
    }
}

struct App {
    cfg: Config,
    manifest: Manifest,
    pet: Pet,
    anim: Animator,
    window: Option<Arc<Window>>,
    gpu: Option<WindowGpu>,
    tray: Option<Tray>,
    proxy: EventLoopProxy<AppEvent>,
    /// 커서가 캐릭터 위에 있어서 창이 클릭을 받는 중.
    hover: bool,
    /// 캐릭터를 끌어 옮기는 중 (왼쪽 버튼을 뗄 때까지).
    dragging: bool,
    /// 끌기 전에 맨 앞에 있던 앱. 끌고 나면 포커스를 돌려준다.
    prev_app: Option<objc2::rc::Retained<objc2_app_kit::NSRunningApplication>>,
    tap_ok: bool,
    next_tap_retry: Option<Instant>,
    redraw_at: Option<Instant>,
    last_frame: Option<Instant>,
    frames: u64,
    /// 벤치마크: 이 시각까지 계속 그린다 (TYPING_PET_BENCH=초).
    bench_until: Option<Instant>,
}

impl App {
    fn new(cfg: Config, manifest: Manifest, proxy: EventLoopProxy<AppEvent>) -> Self {
        let now = Instant::now();
        let pet = Pet::new(manifest.timing.clone(), manifest.keys.clone(), now);
        let mut anim = Animator::new(&manifest, cfg.size as f32, now);
        anim.breathing = cfg.breathing;
        Self {
            cfg,
            manifest,
            pet,
            anim,
            window: None,
            gpu: None,
            tray: None,
            proxy,
            hover: false,
            dragging: false,
            prev_app: None,
            tap_ok: false,
            next_tap_retry: None,
            redraw_at: None,
            last_frame: None,
            frames: 0,
            bench_until: std::env::var("TYPING_PET_BENCH")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .map(|s| Instant::now() + Duration::from_secs_f64(s + 1.0)),
        }
    }

    fn start_tap(&mut self) {
        // 권한이 없어도 탭 생성은 성공하지만 이벤트가 오지 않는다. 권한부터 확인하고, 생길 때까지 3초마다 다시 본다
        if !input::has_listen_permission() {
            log::warn!("입력 모니터링 권한 없음 (메뉴바에 경고 표시, 권한이 생기면 자동으로 시작)");
            self.tap_ok = false;
            self.next_tap_retry = Some(Instant::now() + Duration::from_secs(3));
            if let Some(t) = &mut self.tray {
                t.set_permission_warning(true);
            }
            return;
        }
        let proxy = self.proxy.clone();
        let proxy_c = self.proxy.clone();
        let throttle = Duration::from_millis(self.manifest.timing.mouse_throttle_ms);
        match input::spawn(
            throttle,
            move |i, at| {
                let _ = proxy.send_event(AppEvent::Input(i, at));
            },
            move |c| {
                let _ = proxy_c.send_event(AppEvent::Cursor(c));
            },
        ) {
            Ok(()) => {
                log::info!("입력 감지 시작");
                self.tap_ok = true;
                self.next_tap_retry = None;
            }
            Err(e) => {
                log::warn!("{e}");
                self.tap_ok = false;
                self.next_tap_retry = Some(Instant::now() + Duration::from_secs(3));
            }
        }
        if let Some(t) = &mut self.tray {
            t.set_permission_warning(!self.tap_ok);
        }
    }

    fn create_window(&mut self, el: &ActiveEventLoop) {
        let size = self.cfg.size;
        let pos = self.initial_position(el, size);
        let attrs = Window::default_attributes()
            .with_title("typing-pet")
            .with_transparent(true)
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(size, size))
            .with_position(pos)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_has_shadow(false)
            .with_visible(self.cfg.visible);
        let window = Arc::new(el.create_window(attrs).expect("창 생성 실패"));
        mac::setup_overlay(&window);
        let _ = window.set_cursor_hittest(false);
        log::info!("창 번호 {:?}", mac::window_number(&window));
        match WindowGpu::new(window.clone(), &self.manifest) {
            Ok(g) => self.gpu = Some(g),
            Err(e) => {
                log::error!("렌더러 초기화 실패: {e}");
                el.exit();
            }
        }
        self.window = Some(window);
    }

    /// 저장된 위치가 지금 연결된 화면 밖이면 주 화면 우하단.
    fn initial_position(&self, el: &ActiveEventLoop, size: f64) -> LogicalPosition<f64> {
        let rects: Vec<(f64, f64, f64, f64)> = el
            .available_monitors()
            .map(|m| {
                let s = m.scale_factor();
                let p = m.position().to_logical::<f64>(s);
                let z = m.size().to_logical::<f64>(s);
                (p.x, p.y, z.width, z.height)
            })
            .collect();
        if let Some((x, y)) = self.cfg.position {
            // 창의 절반 이상이 어떤 화면 안에 있으면 그대로 쓴다
            let visible = rects.iter().any(|(rx, ry, rw, rh)| {
                let ix = (x + size).min(rx + rw) - x.max(*rx);
                let iy = (y + size).min(ry + rh) - y.max(*ry);
                ix > size * 0.5 && iy > size * 0.5
            });
            if visible {
                return LogicalPosition::new(x, y);
            }
            log::info!("저장된 위치 ({x}, {y}) 가 화면 밖이라 우하단으로 옮김");
        }
        let (px, py, pw, ph) = el
            .primary_monitor()
            .map(|m| {
                let s = m.scale_factor();
                let p = m.position().to_logical::<f64>(s);
                let z = m.size().to_logical::<f64>(s);
                (p.x, p.y, z.width, z.height)
            })
            .or_else(|| rects.first().copied())
            .unwrap_or((0.0, 0.0, 1440.0, 900.0));
        LogicalPosition::new(px + pw - size - 24.0, py + ph - size - 96.0)
    }

    /// 상태 반영: 바뀐 게 있으면 연출에 넘기고 다시 그리기 요청.
    fn sync(&mut self, now: Instant) {
        self.pet.advance(now);
        let st = self.pet.state(now);
        let events = self.pet.drain_events();
        if st != self.anim.state() || !events.is_empty() {
            trace::event(now, "state", || state_json(&st, self.pet.score(now)));
            self.anim.update(st, &events, now);
            self.request_redraw();
        }
    }

    /// 커서가 캐릭터(불투명한 곳) 위에 있을 때만 창이 클릭을 받게 한다. 나머지는 뒤 앱으로 그대로 간다.
    fn on_cursor(&mut self, c: Cursor) {
        match c {
            Cursor::Up => {
                if self.dragging {
                    self.dragging = false;
                    self.save();
                    if let Some(app) = self.prev_app.take() {
                        mac::activate(&app);
                    }
                }
            }
            Cursor::Move(x, y) => {
                if self.dragging {
                    return;
                }
                let over = match (&self.window, &self.gpu) {
                    (Some(w), Some(g)) if self.cfg.visible => w
                        .outer_position()
                        .map(|p| {
                            let p = p.to_logical::<f64>(w.scale_factor());
                            g.hit.hit((x - p.x) / self.cfg.size, (y - p.y) / self.cfg.size)
                        })
                        .unwrap_or(false),
                    _ => false,
                };
                if over != self.hover {
                    self.hover = over;
                    if over {
                        self.prev_app = mac::frontmost_app();
                    }
                    if let Some(w) = &self.window {
                        let _ = w.set_cursor_hittest(over);
                    }
                }
            }
        }
    }

    fn request_redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let now = Instant::now();
        self.sync(now);
        let Some(gpu) = &mut self.gpu else { return };
        let frame = self.anim.frame(now);
        let t_acq = Instant::now();
        let tex = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            other => {
                log::debug!("surface 텍스처 없음: {other:?}");
                gpu.surface.configure(&gpu.renderer.device, &gpu.config);
                self.request_redraw();
                return;
            }
        };
        let acquired = Instant::now();
        let view = tex.texture.create_view(&Default::default());
        gpu.renderer.draw(&frame, &view);
        let drawn = Instant::now();
        if let Some(w) = &self.window {
            w.pre_present_notify();
        }
        gpu.renderer.queue.present(tex);
        let done = Instant::now();
        let gap = self.last_frame.map(|t| now.saturating_duration_since(t).as_micros() as u64);
        trace::event(now, "frame", || {
            serde_json::json!({
                "n": self.frames,
                "draw_us": done.saturating_duration_since(now).as_micros() as u64,
                "acquire_us": acquired.saturating_duration_since(t_acq).as_micros() as u64,
                "encode_us": drawn.saturating_duration_since(acquired).as_micros() as u64,
                "gap_us": gap,
                "animating": self.anim.animating(now),
                "layers": frame.layer_names(),
            })
        });
        self.frames += 1;
        self.last_frame = Some(now);
        if std::env::var_os("TYPING_PET_BENCH_LOOP").is_some() && self.bench_until.is_some() && self.frames > 5 {
            // 실험: winit 의 다시 그리기 요청을 거치지 않고 연속으로 그림
            let end = self.bench_until.unwrap_or(now);
            while Instant::now() < end {
                let t = Instant::now();
                let gpu = self.gpu.as_mut().unwrap();
                // TYPING_PET_BENCH_EMPTY: 지우기만 하는 기준 측정 (시스템이 줄 수 있는 프레임 간격)
                let frame = if std::env::var_os("TYPING_PET_BENCH_EMPTY").is_some() { Default::default() } else { self.anim.frame(t) };
                if let wgpu::CurrentSurfaceTexture::Success(tex) | wgpu::CurrentSurfaceTexture::Suboptimal(tex) = gpu.surface.get_current_texture() {
                    let view = tex.texture.create_view(&Default::default());
                    gpu.renderer.draw(&frame, &view);
                    gpu.renderer.queue.present(tex);
                }
                let gap = self.last_frame.map(|l| t.saturating_duration_since(l).as_micros() as u64);
                trace::event(t, "frame", || serde_json::json!({"gap_us": gap, "draw_us": t.elapsed().as_micros() as u64}));
                self.last_frame = Some(t);
            }
            trace::flush();
            std::process::exit(0);
        }
        if let Some(t) = self.bench_until {
            if now >= t {
                trace::flush();
                std::process::exit(0);
            }
            self.request_redraw();
            return;
        }
        if self.anim.animating(now) {
            // 연출 중에는 vsync 에 맞춰 계속 그린다
            self.request_redraw();
        } else {
            self.last_frame = None;
        }
    }

    fn on_tray(&mut self, el: &ActiveEventLoop, a: TrayAction) {
        match a {
            TrayAction::ToggleVisible => {
                self.cfg.visible = !self.cfg.visible;
                if let Some(w) = &self.window {
                    w.set_visible(self.cfg.visible);
                    if !self.cfg.visible && self.hover {
                        self.hover = false;
                        let _ = w.set_cursor_hittest(false);
                    }
                }
                self.save();
            }
            TrayAction::ToggleBreathing => {
                self.cfg.breathing = !self.cfg.breathing;
                self.anim.breathing = self.cfg.breathing;
                self.request_redraw();
                self.save();
            }
            TrayAction::Size(s) => {
                self.cfg.size = s;
                self.anim.set_size_pt(self.manifest.canvas, s as f32);
                if let Some(w) = &self.window {
                    let _ = w.request_inner_size(LogicalSize::new(s, s));
                }
                self.save();
            }
            TrayAction::ToggleLoginItem => {
                if let Err(e) = mac::set_login_item(!mac::login_item_enabled()) {
                    log::warn!("로그인 항목 변경 실패: {e}");
                }
            }
            TrayAction::OpenPermissionSettings => mac::open_input_monitoring_settings(),
            TrayAction::Quit => {
                self.save();
                el.exit();
            }
        }
        if let Some(t) = &self.tray {
            t.sync(&self.cfg);
        }
    }

    fn save(&self) {
        if let Err(e) = self.cfg.save() {
            log::warn!("설정 저장 실패: {e}");
        }
    }

    fn schedule(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        self.redraw_at = self.anim.next_redraw(now);
        let wake = [self.pet.next_deadline(now), self.redraw_at, self.next_tap_retry].into_iter().flatten().min();
        el.set_control_flow(match wake {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }
}

fn state_json(st: &PetState, score: f64) -> serde_json::Value {
    let mut v = serde_json::to_value(st).unwrap_or_default();
    v["score"] = serde_json::json!((score * 100.0).round() / 100.0);
    v
}

impl ApplicationHandler<AppEvent> for App {
    fn new_events(&mut self, el: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            let proxy = self.proxy.clone();
            match Tray::new(&self.cfg, move |a| {
                let _ = proxy.send_event(AppEvent::Tray(a));
            }) {
                Ok(t) => self.tray = Some(t),
                Err(e) => log::warn!("메뉴바 아이콘 생성 실패: {e}"),
            }
            if !input::has_listen_permission() {
                log::warn!("입력 모니터링 권한 없음. 권한 요청");
                input::request_listen_permission();
            }
            if std::env::var_os("TYPING_PET_NO_INPUT").is_some() {
                log::info!("TYPING_PET_NO_INPUT: 입력 감지 끔 (CPU 측정용)");
            } else {
                self.start_tap();
            }
            if self.window.is_none() {
                self.create_window(el);
            }
        }
        if let StartCause::ResumeTimeReached { .. } = cause {
            let now = Instant::now();
            if self.next_tap_retry.is_some_and(|t| t <= now) {
                self.start_tap();
            }
            if self.redraw_at.is_some_and(|t| t <= now) {
                self.request_redraw();
            }
        }
    }

    fn resumed(&mut self, _el: &ActiveEventLoop) {}

    fn user_event(&mut self, el: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Input(i, at) => {
                trace::event(at, "input", || serde_json::json!({ "input": i }));
                self.pet.on_input(i, at);
                self.sync(Instant::now());
            }
            AppEvent::Cursor(c) => self.on_cursor(c),
            AppEvent::Tray(a) => self.on_tray(el, a),
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.save();
                el.exit();
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gpu {
                    g.resize(size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                // 배율이 바뀌면 Resized 가 뒤따라 오고, 그때 텍스처를 새 픽셀 크기로 다시 만든다
                self.request_redraw();
            }
            WindowEvent::Moved(pos) => {
                if let Some(w) = &self.window {
                    let p = pos.to_logical::<f64>(w.scale_factor());
                    self.cfg.position = Some((p.x, p.y));
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } if self.hover => {
                if let Some(w) = &self.window {
                    self.dragging = true;
                    let _ = w.drag_window();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.sync(Instant::now());
        self.schedule(el);
    }

    fn exiting(&mut self, _el: &ActiveEventLoop) {
        trace::flush();
    }
}

/// `--render-state <json> --out <png> [--size px] [--wiggle]` 또는 `--render-all <dir> [--size px]`.
fn cli_render(args: &[String]) -> Option<Result<(), String>> {
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let state_arg = get("--render-state");
    let all_arg = get("--render-all");
    let tl_arg = get("--render-timeline");
    if state_arg.is_none() && all_arg.is_none() && tl_arg.is_none() {
        return None;
    }
    Some((|| {
        let manifest = match get("--character-dir") {
            Some(d) => Manifest::load_dir(std::path::Path::new(&d)),
            None => Manifest::load(get("--character").as_deref()),
        }
        .map_err(|e| e.to_string())?;
        let size: u32 = get("--size").map(|s| s.parse().map_err(|_| "--size")).transpose()?.unwrap_or(480);
        let mut off = render::Offscreen::new(&manifest, size)?;
        if let Some(json) = state_arg {
            let st: PetState = serde_json::from_str(&json).map_err(|e| format!("상태 JSON: {e}"))?;
            let out = get("--out").ok_or("--out 필요")?;
            let frame = typing_pet::settled_frame(&manifest, st, args.iter().any(|a| a == "--wiggle"));
            off.render(&frame).save(&out).map_err(|e| e.to_string())?;
            println!("{out}");
        }
        if let Some(dir) = all_arg {
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            for (name, st, wiggle) in typing_pet::all_states() {
                let frame = typing_pet::settled_frame(&manifest, st, wiggle);
                let path = format!("{dir}/{name}.png");
                off.render(&frame).save(&path).map_err(|e| e.to_string())?;
            }
            println!("{dir}");
        }
        if let Some(dir) = tl_arg {
            // 시나리오 입력을 실제 상태 머신/연출에 넣고 fps 간격으로 그린다
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let fps: u64 = get("--fps").and_then(|s| s.parse().ok()).unwrap_or(30);
            let secs: u64 = get("--secs").and_then(|s| s.parse().ok()).unwrap_or(16);
            let t0 = Instant::now();
            let mut pet = Pet::new(manifest.timing.clone(), manifest.keys.clone(), t0);
            let mut anim = Animator::new(&manifest, 240.0, t0);
            let inputs = typing_pet::timeline_inputs();
            let mut next = 0;
            let mut log = Vec::new();
            for f in 0..fps * secs {
                let now_ms = f * 1000 / fps;
                while next < inputs.len() && inputs[next].0 <= now_ms {
                    let (ms, i) = inputs[next];
                    pet.on_input(i, t0 + Duration::from_millis(ms));
                    let at = t0 + Duration::from_millis(ms);
                    pet.advance(at);
                    let ev = pet.drain_events();
                    anim.update(pet.state(at), &ev, at);
                    next += 1;
                }
                let now = t0 + Duration::from_millis(now_ms);
                pet.advance(now);
                let st = pet.state(now);
                let ev = pet.drain_events();
                anim.update(st, &ev, now);
                off.render(&anim.frame(now)).save(format!("{dir}/{f:05}.png")).map_err(|e| e.to_string())?;
                log.push(serde_json::json!({"ms": now_ms, "state": st, "score": (pet.score(now) * 10.0).round() / 10.0}));
            }
            std::fs::write(format!("{dir}/timeline.json"), serde_json::to_string(&log).unwrap_or_default()).map_err(|e| e.to_string())?;
            println!("{dir}");
        }
        Ok(())
    })())
}

fn main() {
    let mut logger = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    // .app 으로 실행하면 stderr 가 없으므로 ~/Library/Logs/typing-pet.log 에 남긴다
    if mac::bundle_path().is_some() {
        if let Some(f) = dirs::home_dir()
            .map(|h| h.join("Library/Logs/typing-pet.log"))
            .and_then(|p| std::fs::File::create(p).ok())
        {
            logger.target(env_logger::Target::Pipe(Box::new(f)));
        }
    }
    logger.init();
    let args: Vec<String> = std::env::args().collect();
    // --login-item on|off: 로그인 시 자동 실행 켜기/끄기 (메뉴와 같은 동작, .app 번들에서만 켤 수 있음)
    if let Some(i) = args.iter().position(|a| a == "--login-item") {
        let on = args.get(i + 1).map(String::as_str) == Some("on");
        match mac::set_login_item(on) {
            Ok(()) => println!("로그인 항목: {}", if mac::login_item_enabled() { "켜짐" } else { "꺼짐" }),
            Err(e) => {
                eprintln!("로그인 항목 변경 실패: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if let Some(r) = cli_render(&args) {
        if let Err(e) = r {
            eprintln!("렌더 실패: {e}");
            std::process::exit(1);
        }
        return;
    }

    trace::init(Instant::now());
    let cfg = Config::load();
    let manifest = Manifest::load(cfg.character.as_deref()).unwrap_or_else(|e| {
        eprintln!("manifest 로딩 실패: {e}");
        std::process::exit(1);
    });
    log::info!("캐릭터 {} ({}), 레이어 {}개", manifest.name, manifest.dir.display(), manifest.layers.len());

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .with_activation_policy(ActivationPolicy::Accessory)
        .build()
        .expect("이벤트 루프 생성 실패");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(cfg, manifest, event_loop.create_proxy());
    event_loop.run_app(&mut app).expect("이벤트 루프 실행 실패");
}
