//! typing-pet 진입점: 이벤트 루프와 창 생성.
//!
//! 흐름:
//! 입력 이벤트 -> 손 상태 결정 -> 표정/효과 결정 -> 프레임 선택 -> 다시 그리기 요청

mod config;
mod input;
mod pet;
mod render;
mod tray;

use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::config::{Config, Manifest};
use crate::input::InputEvent;
use crate::pet::Pet;
use crate::render::Renderer;

/// 입력 훅 스레드에서 이벤트 루프로 넘기는 사용자 이벤트.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Input(InputEvent),
    Tray(tray::TrayAction),
}

struct App {
    config: Config,
    manifest: Manifest,
    pet: Pet,
    window: Option<Window>,
    renderer: Option<Renderer>,
}

impl App {
    fn new(config: Config, manifest: Manifest) -> Self {
        let pet = Pet::new(&manifest);
        Self {
            config,
            manifest,
            pet,
            window: None,
            renderer: None,
        }
    }

    /// 다음 복귀 타이머가 있으면 WaitUntil, 없으면 Wait.
    fn update_control_flow(&self, event_loop: &ActiveEventLoop) {
        match self.pet.next_deadline() {
            Some(t) => event_loop.set_control_flow(ControlFlow::WaitUntil(t)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // TODO: 투명/테두리 없음/항상 위 창 생성, 클릭 통과, Dock 숨김
        // TODO: Renderer::new 로 텍스처 업로드
        let _ = (&self.config, &self.manifest, &self.window, &self.renderer);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::Input(ev) => self.pet.on_input(ev, Instant::now()),
            AppEvent::Tray(action) => {
                // TODO: 보이기/숨기기, 위치 편집, 종료
                log::debug!("tray action: {action:?}");
            }
        }
        // TODO: 프레임이 바뀌었을 때만 request_redraw
        self.update_control_flow(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                // TODO: renderer.draw(self.pet.frame())
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.pet.tick(Instant::now());
        self.update_control_flow(event_loop);
    }
}

fn main() {
    env_logger::init();

    let config = Config::load();
    let manifest = Manifest::load(config.character.as_deref()).expect("manifest 로딩 실패");

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("이벤트 루프 생성 실패");
    event_loop.set_control_flow(ControlFlow::Wait);

    input::spawn(event_loop.create_proxy());

    let mut app = App::new(config, manifest);
    event_loop.run_app(&mut app).expect("이벤트 루프 실행 실패");
}
