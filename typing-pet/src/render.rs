//! wgpu 렌더링: 레이어 합성, 크로스페이드, 키 빛 (PLAN 3.4, 5.4).
//!
//! - 레이어는 시작할 때 전부 읽어 premultiplied alpha 로 바꿔 둔다.
//! - 표시 픽셀 크기가 정해지면 CPU 에서 Lanczos 로 정확한 크기에 맞춰 리사이즈한 뒤 GPU 에 올린다.
//!   (GPU 의 단순 선형 축소는 외곽선이 거칠어진다.) 크기/배율이 바뀌면 다시 만든다.
//! - 합성은 premultiplied alpha (One, OneMinusSrcAlpha). 감마 공간에서 섞어 원본 그림과 같은 색을 낸다.

use std::collections::HashMap;
use std::sync::Arc;

use fast_image_resize as fr;

use crate::anim::{DrawKind, Frame};
use crate::config::Manifest;

const SHADER: &str = r#"
struct U {
    xf: vec4<f32>,   // anchor.xy, offset.xy (uv)
    p: vec4<f32>,    // scale, opacity, mix_t, mode (0 = 레이어 over, 1 = 키 빛, 2 = 마스크 지우기, 3 = 마스크 더하기, 4 = 휘게 over)
    g: vec4<f32>,    // 빛 중심.xy, 반경 (uv), _ | 휘기: root.xy, tip.xy (uv)
    c: vec4<f32>,    // 빛 색 (premultiplied) | 휘기: d.xy, 반경 안/밖 (uv)
};
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var ta: texture_2d<f32>;
@group(1) @binding(1) var tb: texture_2d<f32>;
@group(1) @binding(2) var smp: sampler;
@group(1) @binding(3) var tm: texture_2d<f32>;

struct VO {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VO {
    var corners = array<vec2<f32>, 6>(
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
        vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let uv = corners[i];
    let p = u.xf.xy + (uv - u.xf.xy) * u.p.x + u.xf.zw;
    var o: VO;
    o.pos = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

// 휘기 세기: root -> tip 축을 따라 0 -> 1, 축에서 멀어지면 0
fn warp_w(uv: vec2<f32>) -> f32 {
    let ab = u.g.zw - u.g.xy;
    let t = dot(uv - u.g.xy, ab) / max(dot(ab, ab), 1e-6);
    let q = u.g.xy + clamp(t, 0.0, 1.0) * ab;
    return smoothstep(0.05, 0.85, t) * (1.0 - smoothstep(u.c.z, u.c.w, distance(uv, q)));
}

@fragment
fn fs(v: VO) -> @location(0) vec4<f32> {
    let mode = u32(u.p.w + 0.5);
    var suv = v.uv;
    if (mode == 4u) {
        suv = v.uv - u.c.xy * warp_w(v.uv);
    }
    let a = textureSample(ta, smp, suv);
    let b = textureSample(tb, smp, suv);
    let m = textureSample(tm, smp, v.uv).r * u.p.y;
    if (mode == 1u) {
        let x = clamp(distance(v.uv, u.g.xy) / u.g.z, 0.0, 1.0);
        let k = (1.0 - x * x);
        return u.c * (k * k) * u.p.y;
    }
    if (mode == 2u) {
        // blend (Zero, OneMinusSrcAlpha): dst *= (1 - m)
        return vec4(0.0, 0.0, 0.0, m);
    }
    if (mode == 3u) {
        // blend (One, One): dst += layer * m
        return mix(a, b, u.p.z) * m;
    }
    return mix(a, b, u.p.z) * u.p.y;
}
"#;

const UNI_STRIDE: u64 = 256;
const MAX_DRAWS: u64 = 64;

/// premultiplied RGBA8 원본.
#[derive(Clone)]
pub struct SrcImage {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

pub fn premultiply(data: &mut [u8]) {
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
}

pub fn unpremultiply(data: &mut [u8]) {
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a == 0 {
            px[..3].fill(0);
        } else {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// manifest 의 레이어를 전부 읽는다. 없는 파일은 경고하고 건너뛴다.
pub fn load_layers(m: &Manifest) -> HashMap<String, Arc<SrcImage>> {
    let mut out = HashMap::new();
    for name in &m.layers {
        let path = m.layer_path(name);
        match image::open(&path) {
            Ok(img) => {
                let img = img.to_rgba8();
                let (w, h) = img.dimensions();
                if w != m.canvas || h != m.canvas {
                    log::warn!("{}: 크기 {w}x{h} 가 canvas {} 와 다름", path.display(), m.canvas);
                }
                let mut data = img.into_raw();
                premultiply(&mut data);
                out.insert(name.clone(), Arc::new(SrcImage { w, h, data }));
            }
            Err(e) => log::warn!("레이어 로딩 실패 {}: {e}", path.display()),
        }
    }
    out
}

/// premultiplied 이미지를 Lanczos3 로 정확한 픽셀 크기에 맞춘다.
pub fn resize(src: &SrcImage, size: u32) -> Vec<u8> {
    if src.w == size && src.h == size {
        return src.data.clone();
    }
    let s = fr::images::ImageRef::new(src.w, src.h, &src.data, fr::PixelType::U8x4).expect("이미지 크기");
    let mut dst = fr::images::Image::new(size, size, fr::PixelType::U8x4);
    let mut r = fr::Resizer::new();
    let opt = fr::ResizeOptions::new().resize_alg(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3)).use_alpha(false);
    r.resize(&s, &mut dst, &opt).expect("리사이즈");
    dst.into_vec()
}

struct Tex {
    _tex: wgpu::Texture,
    view: wgpu::TextureView,
}

/// 레이어 텍스처와 파이프라인. 창(surface)과 오프스크린 둘 다 이걸로 그린다.
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    pipe_erase: wgpu::RenderPipeline,
    pipe_add: wgpu::RenderPipeline,
    bgl_tex: wgpu::BindGroupLayout,
    uni_buf: wgpu::Buffer,
    uni_bg: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    src: HashMap<String, Arc<SrcImage>>,
    tex: HashMap<String, Tex>,
    dummy: Tex,
    white: Tex,
    tex_size: u32,
    bind_cache: HashMap<(String, String, String), wgpu::BindGroup>,
    canvas: u32,
}

impl Renderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        src: HashMap<String, Arc<SrcImage>>,
        canvas: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pet"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bgl_uni = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uni"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let bgl_tex = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tex"),
            entries: &[
                tex_entry(0),
                tex_entry(1),
                tex_entry(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pet"),
            bind_group_layouts: &[Some(&bgl_uni), Some(&bgl_tex)],
            immediate_size: 0,
        });
        let make = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend: Some(blend), write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make("over", wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let erase = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let pipe_erase = make("erase", wgpu::BlendState { color: erase, alpha: erase });
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipe_add = make("add", wgpu::BlendState { color: add, alpha: add });
        let uni_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uni"),
            size: UNI_STRIDE * MAX_DRAWS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uni_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uni"),
            layout: &bgl_uni,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uni_buf,
                    offset: 0,
                    size: wgpu::BufferSize::new(64),
                }),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dummy = Self::upload(&device, &queue, 1, &[0, 0, 0, 0], "dummy");
        let white = Self::upload(&device, &queue, 1, &[255, 255, 255, 255], "white");
        Self {
            device,
            queue,
            format,
            pipeline,
            pipe_erase,
            pipe_add,
            bgl_tex,
            uni_buf,
            uni_bg,
            sampler,
            src,
            tex: HashMap::new(),
            dummy,
            white,
            tex_size: 0,
            bind_cache: HashMap::new(),
            canvas,
        }
    }

    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, size: u32, data: &[u8], label: &str) -> Tex {
        let extent = wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 };
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * size), rows_per_image: Some(size) },
            extent,
        );
        let view = tex.create_view(&Default::default());
        Tex { _tex: tex, view }
    }

    pub fn layer_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.src.keys().cloned().collect();
        v.sort();
        v
    }

    /// 표시 픽셀 크기에 맞춰 텍스처를 (다시) 만든다. 같은 크기면 아무것도 안 한다.
    pub fn prepare(&mut self, size_px: u32) {
        if size_px == self.tex_size || size_px == 0 {
            return;
        }
        let t0 = std::time::Instant::now();
        // 레이어마다 독립이라 스레드로 나눠 리사이즈
        let jobs: Vec<(String, Arc<SrcImage>)> = self.src.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let resized: Vec<(String, Vec<u8>)> = std::thread::scope(|s| {
            let hs: Vec<_> = jobs.iter().map(|(k, v)| s.spawn(move || (k.clone(), resize(v, size_px)))).collect();
            hs.into_iter().map(|h| h.join().expect("resize thread")).collect()
        });
        self.tex.clear();
        self.bind_cache.clear();
        for (name, data) in resized {
            let t = Self::upload(&self.device, &self.queue, size_px, &data, &name);
            self.tex.insert(name, t);
        }
        self.tex_size = size_px;
        log::info!("텍스처 {}장을 {size_px}px 로 준비 ({:?})", self.tex.len(), t0.elapsed());
    }

    fn bind(&mut self, a: &str, b: &str, m: &str) -> Option<wgpu::BindGroup> {
        let key = (a.to_string(), b.to_string(), m.to_string());
        if let Some(bg) = self.bind_cache.get(&key) {
            return Some(bg.clone());
        }
        let va = if a.is_empty() { &self.dummy.view } else { &self.tex.get(a)?.view };
        let vb = if b.is_empty() { &self.dummy.view } else { &self.tex.get(b)?.view };
        let vm = if m.is_empty() { &self.white.view } else { &self.tex.get(m)?.view };
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(va) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(vb) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(vm) },
            ],
        });
        self.bind_cache.insert(key, bg.clone());
        Some(bg)
    }

    /// Frame 을 target 에 그린다 (투명으로 지운 뒤).
    pub fn draw(&mut self, frame: &Frame, target: &wgpu::TextureView) {
        let c = self.canvas as f32;
        let mut unis: Vec<[f32; 16]> = Vec::new();
        let mut binds: Vec<(u8, wgpu::BindGroup)> = Vec::new();
        for cmd in &frame.cmds {
            let (a, b, t, mode, g) = match &cmd.kind {
                DrawKind::Layer { name } if cmd.warp.is_some() => {
                    let w = cmd.warp.as_ref().map(|w| [w.root[0] / c, w.root[1] / c, w.tip[0] / c, w.tip[1] / c]).unwrap_or_default();
                    (name.as_str(), name.as_str(), 0.0, 4.0, w)
                }
                DrawKind::Layer { name } => (name.as_str(), name.as_str(), 0.0, 0.0, [0.0; 4]),
                DrawKind::Mix { a, b, t } => (a.as_str(), b.as_str(), *t, 0.0, [0.0; 4]),
                DrawKind::Glow { center, radius, .. } => ("", "", 0.0, 1.0, [center[0] / c, center[1] / c, radius / c, 0.0]),
            };
            let m = cmd.mask.as_deref().unwrap_or("");
            let Some(bg) = self.bind(a, b, m) else {
                log::warn!("텍스처 없음: {a} / {b} / {m}");
                continue;
            };
            let col = match (&cmd.kind, &cmd.warp) {
                (DrawKind::Glow { color, .. }, _) => [color[0] * color[3], color[1] * color[3], color[2] * color[3], color[3]],
                (_, Some(w)) => [w.d[0] / c, w.d[1] / c, w.radius[0] / c, w.radius[1] / c],
                _ => [0.0; 4],
            };
            let u = |mode: f32| {
                [
                    cmd.anchor[0] / c, cmd.anchor[1] / c, cmd.offset[0] / c, cmd.offset[1] / c,
                    cmd.scale, cmd.opacity, t, mode,
                    g[0], g[1], g[2], g[3],
                    col[0], col[1], col[2], col[3],
                ]
            };
            if !m.is_empty() && mode == 0.0 {
                // 마스크 안을 교체: 지우고(dst *= 1-m) 더하기(dst += layer*m)
                unis.push(u(2.0));
                binds.push((1, bg.clone()));
                unis.push(u(3.0));
                binds.push((2, bg));
            } else {
                unis.push(u(mode));
                binds.push((0, bg));
            }
        }
        unis.truncate(MAX_DRAWS as usize);
        binds.truncate(MAX_DRAWS as usize);
        let mut bytes = vec![0u8; (UNI_STRIDE as usize) * unis.len().max(1)];
        for (i, u) in unis.iter().enumerate() {
            let off = i * UNI_STRIDE as usize;
            for (j, f) in u.iter().enumerate() {
                bytes[off + j * 4..off + j * 4 + 4].copy_from_slice(&f.to_le_bytes());
            }
        }
        self.queue.write_buffer(&self.uni_buf, 0, &bytes);
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pet"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for (i, (pipe, bg)) in binds.iter().enumerate().take(MAX_DRAWS as usize) {
                pass.set_pipeline(match pipe {
                    1 => &self.pipe_erase,
                    2 => &self.pipe_add,
                    _ => &self.pipeline,
                });
                pass.set_bind_group(0, &self.uni_bg, &[(i as u64 * UNI_STRIDE) as u32]);
                pass.set_bind_group(1, bg, &[]);
                pass.draw(0..6, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
    }
}

/// 헤드리스 장치 (오프스크린 렌더, 테스트용).
pub fn headless_device() -> Result<(wgpu::Device, wgpu::Queue), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        .map_err(|e| format!("어댑터 없음: {e}"))?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| e.to_string())
}

/// 오프스크린으로 그려 RGBA (straight alpha) 이미지로 돌려준다.
pub struct Offscreen {
    pub renderer: Renderer,
    size: u32,
    target: wgpu::Texture,
}

impl Offscreen {
    pub fn new(m: &Manifest, size: u32) -> Result<Self, String> {
        let (device, queue) = headless_device()?;
        let mut renderer = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm, load_layers(m), m.canvas);
        renderer.prepare(size);
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Self { renderer, size, target })
    }

    pub fn render(&mut self, frame: &Frame) -> image::RgbaImage {
        let view = self.target.create_view(&Default::default());
        self.renderer.draw(frame, &view);
        let size = self.size;
        let row = (4 * size).div_ceil(256) * 256;
        let dev = &self.renderer.device;
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * size) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = dev.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(size) },
            },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        self.renderer.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        dev.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let mapped = slice.get_mapped_range().expect("mapped");
        let mut out = Vec::with_capacity((4 * size * size) as usize);
        for y in 0..size {
            let s = (y * row) as usize;
            out.extend_from_slice(&mapped[s..s + (4 * size) as usize]);
        }
        drop(mapped);
        buf.unmap();
        unpremultiply(&mut out);
        image::RgbaImage::from_raw(size, size, out).expect("이미지")
    }
}
