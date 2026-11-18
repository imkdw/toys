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
    p: vec4<f32>,    // scale, opacity, mix_t, mode (0 = 레이어 over, 1 = 키 빛, 2 = 마스크 지우기, 3 = 마스크 더하기, 4 = 휘게 over, 5 = 구름 흩어지기)
    g: vec4<f32>,    // 빛 중심.xy, 반경 (uv), _ | 휘기: root.xy, tip.xy (uv) | 구름: 흩어짐 0~1, seed, 가로/세로 비율, _
    c: vec4<f32>,    // 빛 색 (premultiplied) | 휘기: d.xy, 반경 안/밖 (uv)
    k: vec4<f32>,    // 가로/세로 배율 (squash), 변형장 사용(1), _
    db: vec4<f32>,   // 변형: 몸 anchor.xy, offset.xy (uv)
    ds: vec4<f32>,   // 변형: 몸 배율.xy, 머리 배율.xy
    dh: vec4<f32>,   // 변형: 머리 anchor.xy, offset.xy (uv)
    dm: vec4<f32>,   // (안 씀)
    bn: vec4<f32>,   // 팔 기울이기: root.xy, tip.xy (uv)
    bd: vec4<f32>,   // 팔 기울이기: 손 이동.xy (uv), 손이 시작하는 곳 (0~1), 사용(1)
};
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var ta: texture_2d<f32>;
@group(1) @binding(1) var tb: texture_2d<f32>;
@group(1) @binding(2) var smp: sampler;
@group(1) @binding(3) var tm: texture_2d<f32>;
@group(1) @binding(4) var tw: texture_2d<f32>;

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
    let p = u.xf.xy + (uv - u.xf.xy) * u.p.x * u.k.xy + u.xf.zw;
    var o: VO;
    o.pos = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

// 변형장: 출력 자리 uv 에서 읽을 원본 자리. 가중치(R 몸, G 머리, B 마우스)만큼 각 변환의 역을 섞는다.
// 가중치가 연속이라 잘린 경계가 없다 (docs/MOTION_QA.md).
fn deform_uv(uv: vec2<f32>) -> vec2<f32> {
    let w = clamp(textureSampleLevel(tw, smp, uv, 0.0), vec4(0.0), vec4(1.0));
    let pb = u.db.xy + (uv - u.db.xy - u.db.zw) / u.ds.xy;
    let ph = u.dh.xy + (uv - u.dh.xy - u.dh.zw) / u.ds.zw;
    return uv + w.r * (pb - uv) + w.g * (ph - uv);
}

// 팔 기울이기: 출력 자리 uv 에서 읽을 팔 그림 자리. 어깨(root)는 제자리, 손(축의 hand 이후)은 d 만큼,
// 그 사이는 어깨에서 멀수록 많이 옮긴다. 옮긴 뒤의 자리에서 거꾸로 찾으므로 몇 번 되풀이해 맞춘다.
fn bend_uv(uv: vec2<f32>) -> vec2<f32> {
    let ax = u.bn.zw - u.bn.xy;
    let l2 = max(dot(ax, ax), 1e-8);
    var q = uv;
    for (var i = 0; i < 6; i++) {
        let s = clamp(dot(q - u.bn.xy, ax) / l2 / u.bd.z, 0.0, 1.0);
        let t = 0.5 * s + 0.5 * s * s * (3.0 - 2.0 * s);
        q = uv - t * u.bd.xy;
    }
    return q;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), s.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), s.x), s.y);
}

@fragment
fn fs(v: VO) -> @location(0) vec4<f32> {
    let mode = u32(u.p.w + 0.5);
    var suv = v.uv;
    if (u.k.z > 0.5) {
        suv = deform_uv(v.uv);
    }
    if (u.bd.w > 0.5) {
        suv = bend_uv(suv);
    }
    let a = textureSample(ta, smp, suv);
    let b = textureSample(tb, smp, suv);
    let m = textureSample(tm, smp, suv).r * u.p.y;
    if (mode == 1u) {
        let x = clamp(distance(v.uv, u.g.xy) / u.g.z, 0.0, 1.0);
        let k = (1.0 - x * x);
        // 마스크가 있으면 그 밖에만 (앞발 캐릭터 픽셀 위에는 빛을 그리지 않는다)
        let keep = select(1.0, 1.0 - textureSample(tm, smp, suv).r, u.g.w > 0.5);
        return u.c * (k * k) * u.p.y * keep;
    }
    if (mode == 5u) {
        // 구름처럼 흩어지기: 가장자리부터, 뭉게뭉게한 무늬를 따라 지워진다
        let d = u.g.x;
        let q = (v.uv - 0.5) * vec2(u.g.z, 1.0);
        let np = q * 4.5 + vec2(u.g.y * 37.0, u.g.y * 11.0 - d * 1.5);
        let n = vnoise(np) * 0.6 + vnoise(np * 2.7) * 0.4;
        let r = length(vec2(q.x / max(u.g.z * 0.5, 0.01), q.y * 2.0));
        let field = (1.0 - r) * 0.6 + n * 0.7;
        let th = mix(-0.45, 1.35, d);
        return a * u.p.y * smoothstep(th, th + 0.22, field);
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
    add_char_masks(&mut out);
    out
}

/// 앞발 레이어마다 캐릭터 픽셀 마스크 `mask/char/<레이어>` 를 만든다 (키 빛을 앞발 위에 그리지 않으려고, [`crate::anim::char_mask_name`]).
/// 앞발 레이어에는 책상/키보드도 같이 들어 있어서, 캐릭터를 지운 책상(base_clean, 없으면 base)과 다른 픽셀을 캐릭터로 본다.
fn add_char_masks(out: &mut HashMap<String, Arc<SrcImage>>) {
    let Some(desk) = out.get("base_clean").or_else(|| out.get("base")).cloned() else { return };
    let names: Vec<String> = out.keys().filter(|k| k.starts_with("paw_")).cloned().collect();
    for name in names {
        let img = out[&name].clone();
        if img.w != desk.w || img.h != desk.h {
            continue;
        }
        let (w, h) = (img.w as usize, img.h as usize);
        let mut on = vec![false; w * h];
        for (i, o) in on.iter_mut().enumerate() {
            let (p, d) = (&img.data[i * 4..i * 4 + 4], &desk.data[i * 4..i * 4 + 4]);
            let diff = (0..3).map(|k| (p[k] as i32 - d[k] as i32).abs()).max().unwrap_or(0);
            *o = (p[3] > 128 && d[3] < 128) || (p[3] > 128 && diff > 40);
        }
        // 윤곽선 가장자리까지 덮도록 2px 넓힌다
        let r = 2usize;
        let mut data = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let hit = (y.saturating_sub(r)..(y + r + 1).min(h)).any(|yy| (x.saturating_sub(r)..(x + r + 1).min(w)).any(|xx| on[yy * w + xx]));
                if hit {
                    data[(y * w + x) * 4..(y * w + x) * 4 + 4].fill(255);
                }
            }
        }
        out.insert(crate::anim::char_mask_name(&name), Arc::new(SrcImage { w: img.w, h: img.h, data }));
    }
}

/// 캐릭터가 그려질 수 있는 자리 (모든 레이어 알파의 합집합, n x n 칸). 커서가 캐릭터 위에 있는지 볼 때 쓴다.
#[derive(Debug, Clone)]
pub struct HitMap {
    n: usize,
    cells: Vec<bool>,
}

impl HitMap {
    pub fn new(layers: &HashMap<String, Arc<SrcImage>>, n: usize) -> Self {
        let mut cells = vec![false; n * n];
        for (name, img) in layers {
            // 소품은 직군에 따라 보였다 안 보였다 해서 따로 판정한다 (PropHits)
            if name.starts_with("mask/") || name.starts_with("prop/") {
                continue;
            }
            let (w, h) = (img.w as usize, img.h as usize);
            for y in 0..h {
                let cy = y * n / h;
                for x in 0..w {
                    if img.data[(y * w + x) * 4 + 3] > 128 {
                        cells[cy * n + x * n / w] = true;
                    }
                }
            }
        }
        Self { n, cells }
    }

    /// u, v: 창 안 위치 (0~1).
    pub fn hit(&self, u: f64, v: f64) -> bool {
        if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
            return false;
        }
        let (x, y) = ((u * self.n as f64) as usize, (v * self.n as f64) as usize);
        self.cells[y * self.n + x]
    }
}

/// 소품별 클릭 판정 (불투명한 곳, n x n 칸).
#[derive(Debug, Clone, Default)]
pub struct PropHits {
    maps: HashMap<String, HitMap>,
}

impl PropHits {
    pub fn new(layers: &HashMap<String, Arc<SrcImage>>, n: usize) -> Self {
        let maps = layers
            .iter()
            .filter_map(|(k, v)| k.strip_prefix("prop/").map(|id| (id.to_string(), HitMap::new(&HashMap::from([(id.to_string(), v.clone())]), n))))
            .collect();
        Self { maps }
    }

    /// u, v 위치에 있는 소품 (보이는 것 중에서, 나중에 그린 것 우선).
    pub fn at<'a>(&self, visible: impl DoubleEndedIterator<Item = &'a str>, u: f64, v: f64) -> Option<&'a str> {
        visible.rev().find(|id| self.maps.get(*id).is_some_and(|m| m.hit(u, v)))
    }
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
    bind_cache: HashMap<(String, String, String, String), wgpu::BindGroup>,
    canvas: u32,
    /// 키 말풍선: (키 이름, 높이 px) -> (가로/세로 비율, 텍스처 바인드). 없으면(그리기 실패) None.
    clouds: HashMap<(&'static str, u32), Option<(f32, wgpu::BindGroup)>>,
    /// 크기 조절 틀을 그릴 때의 화면 배율 (None 이면 안 그린다).
    crop: Option<f32>,
    /// 크기 조절 틀 텍스처: (창 px 너비, 높이) -> 바인드. 창 크기가 바뀌면 다시 만든다.
    crop_tex: Option<((u32, u32), wgpu::BindGroup)>,
}

/// 창 안 배치: 캐릭터 정사각형은 창 아래쪽에 있고, 위쪽 `pad` px 는 키 말풍선 자리.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub w: u32,
    pub h: u32,
}

impl Layout {
    pub fn square(size: u32) -> Self {
        Self { w: size, h: size }
    }

    /// 캐릭터 정사각형 위 띠 높이 (px).
    pub fn pad(&self) -> u32 {
        self.h.saturating_sub(self.w)
    }
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
                    min_binding_size: wgpu::BufferSize::new(176),
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
                tex_entry(4),
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
                    size: wgpu::BufferSize::new(176),
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
            clouds: HashMap::new(),
            crop: None,
            crop_tex: None,
        }
    }

    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, size: u32, data: &[u8], label: &str) -> Tex {
        Self::upload_wh(device, queue, size, size, data, label)
    }

    fn upload_wh(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, data: &[u8], label: &str) -> Tex {
        let extent = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
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
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
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

    /// 다른 캐릭터로 바꿀 때: 원본 레이어를 갈아 끼우고 지금 표시 크기로 텍스처를 다시 만든다.
    pub fn set_layers(&mut self, src: HashMap<String, Arc<SrcImage>>, canvas: u32) {
        let size = self.tex_size;
        self.src = src;
        self.canvas = canvas;
        self.tex.clear();
        self.bind_cache.clear();
        self.tex_size = 0;
        self.prepare(size);
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

    fn bind(&mut self, a: &str, b: &str, m: &str, w: &str) -> Option<wgpu::BindGroup> {
        let key = (a.to_string(), b.to_string(), m.to_string(), w.to_string());
        if let Some(bg) = self.bind_cache.get(&key) {
            return Some(bg.clone());
        }
        let va = if a.is_empty() { &self.dummy.view } else { &self.tex.get(a)?.view };
        let vb = if b.is_empty() { &self.dummy.view } else { &self.tex.get(b)?.view };
        let vm = if m.is_empty() { &self.white.view } else { &self.tex.get(m)?.view };
        let vw = if w.is_empty() { &self.dummy.view } else { &self.tex.get(w)?.view };
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(va) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(vb) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(vm) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(vw) },
            ],
        });
        self.bind_cache.insert(key, bg.clone());
        Some(bg)
    }

    /// 크기 조절 틀 켜기/끄기. `scale` 은 화면 배율 (선 굵기용).
    pub fn set_crop(&mut self, scale: Option<f32>) {
        if self.crop != scale {
            self.crop = scale;
            self.crop_tex = None;
        }
    }

    /// 창 전체 크기의 크기 조절 틀 텍스처 (창 크기가 같으면 만들어 둔 것).
    fn crop_bind(&mut self, l: Layout, scale: f32) -> wgpu::BindGroup {
        if let Some((wh, bg)) = &self.crop_tex {
            if *wh == (l.w, l.h) {
                return bg.clone();
            }
        }
        #[allow(unused_mut)]
        let mut data = crate::crop::frame_image(l.w, l.h, l.pad(), scale);
        // 안내문: 캐릭터 정사각형 위쪽, 손잡이 아래 가운데
        #[cfg(target_os = "macos")]
        {
            let side = l.w.min(l.h - l.pad()) as i32;
            let hs = crate::crop::handle_px(side, scale);
            let max_w = (side - hs * 4).max(0) as u32;
            if let Some(img) = crate::text::hint(&["모서리를 끌어 크기 조절", "Enter 저장 / Esc 취소"], max_w, scale) {
                let x = (side - img.w as i32) / 2;
                let y = l.pad() as i32 + hs + (6.0 * scale) as i32;
                crate::crop::paste(&mut data, l.w, l.h, &img.data, img.w, img.h, x, y);
            }
        }
        let t = Self::upload_wh(&self.device, &self.queue, l.w, l.h, &data, "crop");
        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crop"),
            layout: &self.bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&t.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&t.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.white.view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.dummy.view) },
            ],
        });
        self.crop_tex = Some(((l.w, l.h), bg.clone()));
        bg
    }

    /// 키 말풍선 텍스처 (처음 쓸 때 그려 둔다).
    fn cloud(&mut self, label: &'static str, h: u32) -> Option<(f32, wgpu::BindGroup)> {
        if let Some(v) = self.clouds.get(&(label, h)) {
            return v.clone();
        }
        // 크기를 자주 바꾸면 쌓이므로 적당히 비운다
        if self.clouds.len() > 256 {
            self.clouds.clear();
        }
        #[cfg(target_os = "macos")]
        let img = crate::text::cloud(label, h);
        #[cfg(not(target_os = "macos"))]
        let img: Option<SrcImage> = None;
        let v = img.map(|img| {
            let t = Self::upload_wh(&self.device, &self.queue, img.w, img.h, &img.data, label);
            let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud"),
                layout: &self.bgl_tex,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&t.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&t.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.white.view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.dummy.view) },
                ],
            });
            (img.w as f32 / img.h as f32, bg)
        });
        if v.is_none() {
            log::warn!("키 말풍선 그리기 실패: {label}");
        }
        self.clouds.insert((label, h), v.clone());
        v
    }

    /// Frame 을 target 에 그린다 (투명으로 지운 뒤). target 은 정사각형.
    pub fn draw(&mut self, frame: &Frame, target: &wgpu::TextureView) {
        self.draw_scene(frame, &[], None, target);
    }

    /// 캐릭터(`frame`)와 키 말풍선을 그린다. `layout` 이 있으면 캐릭터는 아래쪽 정사각형에, 말풍선은 창 전체 기준.
    pub fn draw_scene(&mut self, frame: &Frame, keys: &[crate::keyecho::Sprite], layout: Option<Layout>, target: &wgpu::TextureView) {
        let c = self.canvas as f32;
        let mut unis: Vec<[f32; 44]> = Vec::new();
        // (파이프라인, 바인드, 창 전체에 그리기)
        let mut binds: Vec<(u8, wgpu::BindGroup, bool)> = Vec::new();
        for cmd in &frame.cmds {
            let (a, b, t, mode, g) = match &cmd.kind {
                DrawKind::Layer { name } => (name.as_str(), name.as_str(), 0.0, 0.0, [0.0; 4]),
                DrawKind::Mix { a, b, t } => (a.as_str(), b.as_str(), *t, 0.0, [0.0; 4]),
                DrawKind::Glow { center, radius, .. } => ("", "", 0.0, 1.0, [center[0] / c, center[1] / c, radius / c, if cmd.mask.is_some() { 1.0 } else { 0.0 }]),
            };
            let m = cmd.mask.as_deref().unwrap_or("");
            let wf = cmd.deform.as_ref().map_or("", |d| d.field.as_str());
            let Some(bg) = self.bind(a, b, m, wf) else {
                log::warn!("텍스처 없음: {a} / {b} / {m}");
                continue;
            };
            let col = match &cmd.kind {
                DrawKind::Glow { color, .. } => [color[0] * color[3], color[1] * color[3], color[2] * color[3], color[3]],
                _ => [0.0; 4],
            };
            let dv: [f32; 16] = match &cmd.deform {
                Some(d) => [
                    d.body.anchor[0] / c, d.body.anchor[1] / c, d.body.offset[0] / c, d.body.offset[1] / c,
                    d.body.s[0], d.body.s[1], d.head.s[0], d.head.s[1],
                    d.head.anchor[0] / c, d.head.anchor[1] / c, d.head.offset[0] / c, d.head.offset[1] / c,
                    0.0, 0.0, 0.0, 0.0,
                ],
                None => [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            };
            let on = if cmd.deform.is_some() { 1.0 } else { 0.0 };
            let bv: [f32; 8] = match &cmd.bend {
                Some(b) => [b.root[0] / c, b.root[1] / c, b.tip[0] / c, b.tip[1] / c, b.d[0] / c, b.d[1] / c, b.hand.max(0.05), 1.0],
                None => [0.0; 8],
            };
            let u = |mode: f32| {
                [
                    cmd.anchor[0] / c, cmd.anchor[1] / c, cmd.offset[0] / c, cmd.offset[1] / c,
                    cmd.scale, cmd.opacity, t, mode,
                    g[0], g[1], g[2], g[3],
                    col[0], col[1], col[2], col[3],
                    cmd.squash[0], cmd.squash[1], on, 0.0,
                    dv[0], dv[1], dv[2], dv[3], dv[4], dv[5], dv[6], dv[7],
                    dv[8], dv[9], dv[10], dv[11], dv[12], dv[13], dv[14], dv[15],
                    bv[0], bv[1], bv[2], bv[3], bv[4], bv[5], bv[6], bv[7],
                ]
            };
            if !m.is_empty() && mode == 0.0 {
                // 마스크 안을 교체: 지우고(dst *= 1-m) 더하기(dst += layer*m)
                unis.push(u(2.0));
                binds.push((1, bg.clone(), false));
                unis.push(u(3.0));
                binds.push((2, bg, false));
            } else {
                unis.push(u(mode));
                binds.push((0, bg, false));
            }
        }
        if let Some(l) = layout.filter(|_| !keys.is_empty()) {
            let (w, h, pad) = (l.w as f32, l.h as f32, l.pad() as f32);
            for k in keys {
                // 텍스처는 기본 크기로 한 번만 그리고, 튀어나오기/부풀기는 늘려 그린다
                let tex_h = (crate::keyecho::Sprite::BASE_HEIGHT * w).round().max(8.0) as u32;
                let Some((aspect, bg)) = self.cloud(k.label, tex_h) else { continue };
                let sh = k.height * w;
                let sw = sh * aspect;
                let (cx, cy) = (k.center[0] * w, pad + k.center[1] * w);
                let (kx, ky) = (sw / w, sh / h);
                unis.push([
                    0.0, 0.0, cx / w - kx / 2.0, cy / h - ky / 2.0,
                    1.0, k.opacity, 0.0, 5.0,
                    k.dissolve, k.seed, aspect, 0.0,
                    0.0, 0.0, 0.0, 0.0,
                    kx, ky, 0.0, 0.0,
                    0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                ]);
                binds.push((0, bg, true));
            }
        }
        unis.truncate(MAX_DRAWS as usize - 1);
        binds.truncate(MAX_DRAWS as usize - 1);
        // 크기 조절 틀은 맨 위에 창 전체로 (정사각형 텍스처 그대로: anchor 0, 배율 1)
        if let (Some(l), Some(scale)) = (layout, self.crop) {
            let bg = self.crop_bind(l, scale);
            let mut u = [0.0f32; 44];
            u[4] = 1.0; // scale
            u[5] = 1.0; // opacity
            u[16] = 1.0; // squash x
            u[17] = 1.0; // squash y
            u[24] = 1.0;
            u[25] = 1.0;
            u[26] = 1.0;
            u[27] = 1.0;
            unis.push(u);
            binds.push((0, bg, true));
        }
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
            let mut full = None;
            for (i, (pipe, bg, whole)) in binds.iter().enumerate().take(MAX_DRAWS as usize) {
                if let Some(l) = layout {
                    if full != Some(*whole) {
                        full = Some(*whole);
                        if *whole {
                            pass.set_viewport(0.0, 0.0, l.w as f32, l.h as f32, 0.0, 1.0);
                        } else {
                            pass.set_viewport(0.0, l.pad() as f32, l.w as f32, l.w as f32, 0.0, 1.0);
                        }
                    }
                }
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
    /// 키 말풍선 띠를 포함한 높이 (없으면 size).
    height: u32,
    target: wgpu::Texture,
}

impl Offscreen {
    pub fn new(m: &Manifest, size: u32) -> Result<Self, String> {
        Self::with_pad(m, size, 0)
    }

    /// 캐릭터 정사각형 위에 `pad` px 띠를 더한 판 (키 말풍선 확인용).
    pub fn with_pad(m: &Manifest, size: u32, pad: u32) -> Result<Self, String> {
        let height = size + pad;
        let (device, queue) = headless_device()?;
        let mut renderer = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm, load_layers(m), m.canvas);
        renderer.prepare(size);
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d { width: size, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Self { renderer, size, height, target })
    }

    /// 캐릭터 정사각형 기준 위치/크기 (키 말풍선 띠 포함).
    pub fn layout(&self) -> Layout {
        Layout { w: self.size, h: self.height }
    }

    pub fn render(&mut self, frame: &Frame) -> image::RgbaImage {
        self.render_scene(frame, &[])
    }

    pub fn render_scene(&mut self, frame: &Frame, keys: &[crate::keyecho::Sprite]) -> image::RgbaImage {
        let view = self.target.create_view(&Default::default());
        let layout = self.layout();
        self.renderer.draw_scene(frame, keys, (layout.pad() > 0).then_some(layout), &view);
        let (size, height) = (self.size, self.height);
        let row = (4 * size).div_ceil(256) * 256;
        let dev = &self.renderer.device;
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = dev.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &self.target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) },
            },
            wgpu::Extent3d { width: size, height, depth_or_array_layers: 1 },
        );
        self.renderer.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        dev.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let mapped = slice.get_mapped_range().expect("mapped");
        let mut out = Vec::with_capacity((4 * size * height) as usize);
        for y in 0..height {
            let s = (y * row) as usize;
            out.extend_from_slice(&mapped[s..s + (4 * size) as usize]);
        }
        drop(mapped);
        buf.unmap();
        unpremultiply(&mut out);
        image::RgbaImage::from_raw(size, height, out).expect("이미지")
    }
}

/// 키 말풍선이 나올 자리: 머리(head/normal, 없으면 base) 불투명 영역의 가운데 x 와 꼭대기 y (0~1).
pub fn head_spot(layers: &HashMap<String, Arc<SrcImage>>) -> Option<(f32, f32)> {
    let img = layers.get("head/normal").or_else(|| layers.get("base"))?;
    let (w, h) = (img.w as usize, img.h as usize);
    let (mut x0, mut x1, mut top) = (w, 0, None);
    for y in 0..h {
        for x in 0..w {
            if img.data[(y * w + x) * 4 + 3] > 128 {
                top.get_or_insert(y);
                x0 = x0.min(x);
                x1 = x1.max(x);
            }
        }
    }
    let top = top?;
    Some(((x0 + x1) as f32 / 2.0 / w as f32, top as f32 / h as f32))
}
