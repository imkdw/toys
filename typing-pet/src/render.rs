//! wgpu 렌더링: 전체 프레임 1장 + 오버레이 합성.
//!
//! - 텍스처는 시작할 때 GPU 에 한 번만 올린다. 프레임 전환은 바인드 그룹만 바꾼다.
//! - surface 가 지원하는 alpha 모드를 로그로 확인한다.
//!   투명이 안 되면 softbuffer 또는 objc2 CALayer 로 교체.

use crate::config::Manifest;
use crate::pet::FrameSelection;

pub struct Renderer {
    // TODO: instance, surface, device, queue, pipeline, frame 이름 -> 텍스처 맵
}

impl Renderer {
    pub fn new(manifest: &Manifest) -> Self {
        // TODO: wgpu 초기화, manifest 의 프레임 PNG 로딩/업로드
        let _ = manifest;
        Self {}
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        // TODO
        let _ = (width, height);
    }

    pub fn draw(&mut self, selection: &FrameSelection) {
        // TODO
        let _ = selection;
    }
}
