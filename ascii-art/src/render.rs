//! RGBA 프레임을 글자 격자(글자 + 색)로 바꾸고 터미널 문자열로 인코딩한다.
//!
//! 한 칸은 SX x SY 개의 부분 화소로 디코딩해서 계산한다. 터미널 글자는 세로가 가로의
//! 약 2배라서 SY = SX * 2 이면 부분 화소가 정사각형이 된다. 배경 마스크, 선(잉크) 판정,
//! 입체 음영을 모두 칸보다 촘촘한 이 부분 화소에서 계산한다.
//!
//! 그리는 방식은 두 가지다.
//! - 피사체 모드: 배경을 지울 수 있으면(투명, 단색 배경, 외곽선으로 닫힌 캐릭터) 피사체만 남기고,
//!   어두운 선은 빈칸으로 비우고, 면은 촘촘한 글자로 채우면서 실루엣에서 만든 입체 음영을 입힌다.
//! - 장면 모드: 배경을 지울 수 없는 사진/영상은 화면 전체를 밝기 램프로 채운다.

use std::fmt::Write;

pub const SX: usize = 4;
pub const SY: usize = 8;

/// 면이 칸을 이만큼 덮지 못하면(선이나 배경이 나머지를 차지하면) 그 칸은 빈칸이 된다.
const INK_FULL: f32 = 0.66;

/// 선이 살짝만 걸친 칸에 쓰는 가는 글자. 굵게 덮인 칸(눈동자 등)은 램프의 가장 진한 글자를 쓴다.
const INK_THIN: char = '*';

pub fn ramp_preset(name: &str) -> Option<&'static str> {
    Some(match name {
        "letters" => " .:n0Q8B&#$@",
        "standard" => " .:-=+*#%@",
        "dense" => " .'`^\",:;Il!i><~+_-?][}{1)(|tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$",
        "simple" => " .oO@",
        "blocks" => " ░▒▓█",
        _ => return None,
    })
}

/// 윤곽 방향 글자: 수평, 오른쪽 위, 수직, 왼쪽 위.
const EDGE_GLYPHS: [char; 4] = ['-', '/', '|', '\\'];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BgMode {
    Auto,
    None,
    Alpha,
    /// 어두운 외곽선으로 닫힌 영역만 남긴다 (배경 그림이 있는 애니 장면용)
    Outline,
    Color([f32; 3]),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub ramp: Vec<char>,
    pub contrast: f32,
    pub gamma: f32,
    pub brightness: f32,
    pub invert: bool,
    pub bg: BgMode,
    pub bg_tol: f32,
    /// 이 밝기보다 어두운 화소를 선(잉크)으로 본다. 피사체 모드에서 선은 빈칸이 된다.
    pub ink_thr: f32,
    /// 참이면 선 칸을 빈칸으로 비우고, 거짓이면 선 색 글자로 그린다 (기본).
    pub ink_gap: bool,
    /// 입체 음영 세기 (0 = 끔). None 이면 자동: 피사체 모드 1, 장면 모드 0
    pub shade: Option<f32>,
    pub edges: bool,
    pub edge_thr: f32,
    /// 어두운 칸도 전경색이 이 밝기 아래로 떨어지지 않게 한다.
    pub color_floor: f32,
    /// 히스토그램 평활화 비율 (0 = 끔, 1 = 순위만 사용). None 이면 자동:
    /// 피사체 모드 0, 장면 모드 0.7
    pub equalize: Option<f32>,
    pub bold: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            ramp: ramp_preset("letters").unwrap().chars().collect(),
            contrast: 1.0,
            gamma: 1.0,
            brightness: 0.0,
            invert: false,
            bg: BgMode::Auto,
            bg_tol: 0.12,
            ink_thr: 0.36,
            ink_gap: false,
            shade: None,
            edges: false,
            edge_thr: 0.12,
            color_floor: 0.35,
            equalize: None,
            bold: true,
        }
    }
}

pub fn parse_hex(s: &str) -> Option<[f32; 3]> {
    let s = s.trim_start_matches('#');
    let s: String = if s.len() == 3 { s.chars().flat_map(|c| [c, c]).collect() } else { s.to_string() };
    if s.len() != 6 {
        return None;
    }
    let mut out = [0f32; 3];
    for i in 0..3 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()? as f32 / 255.0;
    }
    Some(out)
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub chars: Vec<char>,
    pub colors: Vec<[u8; 3]>,
    pub bold: bool,
}

impl Grid {
    pub fn nonblank(&self) -> usize {
        self.chars.iter().filter(|&&c| c != ' ').count()
    }
}

#[inline]
fn tone(l: f32, o: &Options) -> f32 {
    let mut v = ((l - 0.5) * o.contrast + 0.5 + o.brightness).clamp(0.0, 1.0);
    if o.gamma != 1.0 {
        v = v.powf(1.0 / o.gamma);
    }
    if o.invert { 1.0 - v } else { v }
}

#[inline]
fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (d0, d1, d2) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (d0 * d0 + d1 * d1 + d2 * d2).sqrt() / 3f32.sqrt()
}

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// 가장자리와 이어진 open 영역만 배경으로 남긴다 (4방향).
fn flood_from_border(open: &[bool], w: usize, h: usize) -> Vec<bool> {
    let mut bg = vec![false; w * h];
    let mut q: Vec<usize> = Vec::new();
    let push = |i: usize, bg: &mut Vec<bool>, q: &mut Vec<usize>| {
        if open[i] && !bg[i] {
            bg[i] = true;
            q.push(i);
        }
    };
    for x in 0..w {
        push(x, &mut bg, &mut q);
        push((h - 1) * w + x, &mut bg, &mut q);
    }
    for y in 0..h {
        push(y * w, &mut bg, &mut q);
        push(y * w + w - 1, &mut bg, &mut q);
    }
    while let Some(i) = q.pop() {
        let (x, y) = (i % w, i / w);
        if x > 0 { push(i - 1, &mut bg, &mut q); }
        if x + 1 < w { push(i + 1, &mut bg, &mut q); }
        if y > 0 { push(i - w, &mut bg, &mut q); }
        if y + 1 < h { push(i + w, &mut bg, &mut q); }
    }
    bg
}

/// 사방 r 화소(정사각형)로 넓힌다. 가로, 세로로 나눠 한 번씩 훑는다.
fn dilate(m: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let pass = |src: &[bool], len: usize, lines: usize, stride: usize, step: usize, out: &mut [bool]| {
        for l in 0..lines {
            let base = l * stride;
            // 가장 가까운 참 화소까지의 거리를 양쪽에서 훑어 구한다
            let mut last = usize::MAX;
            for k in 0..len {
                if src[base + k * step] {
                    last = k;
                }
                out[base + k * step] = last != usize::MAX && k - last <= r;
            }
            last = usize::MAX;
            for k in (0..len).rev() {
                if src[base + k * step] {
                    last = k;
                }
                if last != usize::MAX && last - k <= r {
                    out[base + k * step] = true;
                }
            }
        }
    };
    let mut tmp = vec![false; w * h];
    let mut out = vec![false; w * h];
    pass(m, w, h, w, 1, &mut tmp);
    pass(&tmp, h, w, 1, w, &mut out);
    out
}

/// 어두운 외곽선으로 닫힌 영역을 피사체로 본다: 가장자리에서 밝은 화소를 따라 번져 간 곳이 배경이고,
/// 외곽선에 막혀 닿지 못한 곳이 피사체다. 선만 있고 안쪽 면이 없는 조각(효과선, 풀 등)은 버린다.
/// strict 이면 결과가 그럴듯할 때만 돌려준다 (자동 모드에서 사진을 건드리지 않게).
fn outline_mask(lum: &[f32], w: usize, h: usize, o: &Options, strict: bool) -> Option<Vec<bool>> {
    // 손그림 외곽선은 획 사이가 조금씩 떠 있다. 틈을 메우는 폭을 넓혀 가며 처음으로 닫히는 결과를 쓴다.
    let base = (w.min(h) / 120).max(1);
    [base, base * 2, base * 4].into_iter().find_map(|close| outline_mask_with(lum, w, h, o, strict, close))
}

fn outline_mask_with(lum: &[f32], w: usize, h: usize, o: &Options, strict: bool, close: usize) -> Option<Vec<bool>> {
    let n = w * h;
    let dark: Vec<bool> = lum.iter().map(|&l| l < o.ink_thr).collect();
    // 선의 틈으로 새지 않게 close 화소만큼 두껍게 막고 번진 뒤, 그만큼 되돌린다
    let wall = dilate(&dark, w, h, close);
    let open: Vec<bool> = wall.iter().map(|&d| !d).collect();
    let mut bg = flood_from_border(&open, w, h);
    // 되돌리기: 배경에서 선이 아닌 화소를 따라 close 걸음까지만 번진다 (선을 건너뛰지 않는다)
    let mut front: Vec<usize> = (0..n).filter(|&i| bg[i]).collect();
    for _ in 0..close {
        let mut next = Vec::new();
        for &i in &front {
            let (x, y) = (i % w, i / w);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = yy * w + xx;
                    if !bg[j] && !dark[j] {
                        bg[j] = true;
                        next.push(j);
                    }
                }
            }
        }
        front = next;
    }
    // 연결 조각(8방향)마다 넓이와 밝은 화소 수를 센다
    let mut label = vec![u32::MAX; n];
    let mut comps: Vec<(usize, usize)> = Vec::new();
    let mut stack = Vec::new();
    for s in 0..n {
        if bg[s] || label[s] != u32::MAX {
            continue;
        }
        let id = comps.len() as u32;
        let (mut area, mut light) = (0usize, 0usize);
        label[s] = id;
        stack.push(s);
        while let Some(i) = stack.pop() {
            area += 1;
            if !dark[i] {
                light += 1;
            }
            let (x, y) = (i % w, i / w);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = yy * w + xx;
                    if !bg[j] && label[j] == u32::MAX {
                        label[j] = id;
                        stack.push(j);
                    }
                }
            }
        }
        comps.push((area, light));
    }
    let solid = |c: &(usize, usize)| c.1 * 4 >= c.0;
    let largest = comps.iter().filter(|c| solid(c)).map(|c| c.0).max().unwrap_or(0);
    let keep: Vec<bool> = comps.iter().map(|c| solid(c) && c.0 * 25 >= largest).collect();
    let (mut area, mut light) = (0usize, 0usize);
    for (c, k) in comps.iter().zip(&keep) {
        if *k {
            area += c.0;
            light += c.1;
        }
    }
    if area == 0 {
        return None;
    }
    if strict && (area * 25 < n || area * 20 > n * 17 || light * 2 < area) {
        return None;
    }
    for i in 0..n {
        if !bg[i] && !keep[label[i] as usize] {
            bg[i] = true;
        }
    }
    Some(bg)
}

/// 부분 화소 단위 배경 마스크와 (색으로 뺐다면) 그 배경색. 배경 제거를 하지 않으면 None.
fn background_mask(rgb: &[[f32; 3]], alpha: &[f32], lum: &[f32], w: usize, h: usize, o: &Options) -> Option<(Vec<bool>, Option<[f32; 3]>)> {
    if o.bg == BgMode::None {
        return None;
    }
    if o.bg == BgMode::Outline {
        return outline_mask(lum, w, h, o, false).map(|m| (m, None));
    }
    if matches!(o.bg, BgMode::Auto | BgMode::Alpha) {
        let transparent = alpha.iter().filter(|&&a| a < 0.98).count();
        if transparent * 100 > alpha.len() {
            return Some((alpha.iter().map(|&a| a < 0.5).collect(), None));
        }
        if o.bg == BgMode::Alpha {
            return None;
        }
    }
    let reference = match o.bg {
        BgMode::Color(c) => c,
        _ => {
            let mut border = Vec::with_capacity(2 * (w + h));
            for x in 0..w {
                border.push(rgb[x]);
                border.push(rgb[(h - 1) * w + x]);
            }
            for y in 0..h {
                border.push(rgb[y * w]);
                border.push(rgb[y * w + w - 1]);
            }
            let r = [0, 1, 2].map(|c| median(border.iter().map(|p| p[c]).collect()));
            let near = border.iter().filter(|p| dist(**p, r) < o.bg_tol).count();
            // 가장자리가 거의 단색이 아니면 외곽선으로 닫힌 캐릭터를 찾아 보고, 없으면 장면으로 둔다
            if near * 10 < border.len() * 9 {
                return outline_mask(lum, w, h, o, true).map(|m| (m, None));
            }
            r
        }
    };
    let similar: Vec<bool> = rgb.iter().map(|p| dist(*p, reference) < o.bg_tol).collect();
    Some((flood_from_border(&similar, w, h), Some(reference)))
}

/// 반지름 r 상자 흐림 (가장자리는 끝 값을 이어 붙인다).
fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let pass = |src: &[f32], len: usize, lines: usize, stride: usize, step: usize, out: &mut [f32]| {
        let norm = 1.0 / (2 * r + 1) as f32;
        let mut pre = vec![0f32; len + 1];
        for l in 0..lines {
            let base = l * stride;
            for k in 0..len {
                pre[k + 1] = pre[k] + src[base + k * step];
            }
            let (first, last) = (src[base], src[base + (len - 1) * step]);
            for k in 0..len {
                let lo = k as isize - r as isize;
                let hi = k + r + 1;
                let mut sum = pre[hi.min(len)] - pre[lo.max(0) as usize];
                if lo < 0 {
                    sum += first * (-lo) as f32;
                }
                if hi > len {
                    sum += last * (hi - len) as f32;
                }
                out[base + k * step] = sum * norm;
            }
        }
    };
    let mut tmp = vec![0f32; w * h];
    let mut out = vec![0f32; w * h];
    pass(src, w, h, w, 1, &mut tmp);
    pass(&tmp, h, w, 1, w, &mut out);
    out
}

/// 실루엣을 부풀린 풍선처럼 보고 왼쪽 위에서 비춘 음영(0~1)을 만든다.
/// 여러 크기로 흐린 마스크의 기울기를 더해 법선으로 쓴다: 가장자리는 둥글게 말리고 안쪽은 완만하게 볼록하다.
/// 음영은 완만해서 SX x SX 화소를 한 점으로 줄인 격자(w/SX x h/SX)에서 계산하고 그 크기로 돌려준다.
fn volume_shade(fg: &[bool], w: usize, h: usize) -> (Vec<f32>, usize, usize) {
    let (sw, sh) = (w / SX, h / SX);
    let n = sw * sh;
    let mut mask = vec![0f32; n];
    for y in 0..sh {
        for x in 0..sw {
            let mut c = 0;
            for yy in 0..SX {
                let row = (y * SX + yy) * w + x * SX;
                c += fg[row..row + SX].iter().filter(|&&f| f).count();
            }
            mask[y * sw + x] = c as f32 / (SX * SX) as f32;
        }
    }
    let size = mask.iter().sum::<f32>().sqrt().max(2.0);
    let (mut nx, mut ny) = (vec![0f32; n], vec![0f32; n]);
    for (scale, gain) in [(0.045f32, 0.8f32), (0.13, 0.9), (0.30, 0.9)] {
        let r = ((size * scale).round() as usize).max(1);
        let b = box_blur(&box_blur(&mask, sw, sh, r), sw, sh, r);
        let k = gain * r as f32;
        for y in 0..sh {
            let (y0, y1) = (y.saturating_sub(1), (y + 1).min(sh - 1));
            for x in 0..sw {
                let (x0, x1) = (x.saturating_sub(1), (x + 1).min(sw - 1));
                let i = y * sw + x;
                nx[i] -= k * (b[y * sw + x1] - b[y * sw + x0]) * 0.5;
                ny[i] -= k * (b[y1 * sw + x] - b[y0 * sw + x]) * 0.5;
            }
        }
    }
    // 빛: 왼쪽 위 앞 (화면 y 는 아래로 증가)
    let (lx, ly, lz) = (-0.40f32, -0.50, 0.77);
    let shade = (0..n)
        .map(|i| {
            let inv = 1.0 / (nx[i] * nx[i] + ny[i] * ny[i] + 1.0).sqrt();
            ((nx[i] * lx + ny[i] * ly + lz) * inv).clamp(0.0, 1.0)
        })
        .collect();
    (shade, sw, sh)
}

/// (rows*SY) x (cols*SX) 크기 RGBA 프레임을 글자 격자로 바꾼다.
pub fn render(frame: &[u8], fw: usize, fh: usize, o: &Options) -> Grid {
    let cols = fw / SX;
    let rows = fh / SY;
    let (w, h) = (cols * SX, rows * SY);
    let n = w * h;
    let mut rgb = vec![[0f32; 3]; n];
    let mut alpha = vec![0f32; n];
    let mut lum = vec![0f32; n];
    for y in 0..h {
        for x in 0..w {
            let s = (y * fw + x) * 4;
            let i = y * w + x;
            let p = [frame[s] as f32 / 255.0, frame[s + 1] as f32 / 255.0, frame[s + 2] as f32 / 255.0];
            rgb[i] = p;
            alpha[i] = frame[s + 3] as f32 / 255.0;
            lum[i] = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        }
    }
    let (bg, key) = match background_mask(&rgb, &alpha, &lum, w, h, o) {
        Some((m, k)) => (Some(m), k),
        None => (None, None),
    };
    let has_bg = bg.is_some();
    let fg: Vec<bool> = match &bg {
        Some(b) => b.iter().map(|&v| !v).collect(),
        None => vec![true; n],
    };
    // 색으로 배경을 뺐다면 경계의 안티앨리어싱 화소에 배경색이 번져 있다.
    // 배경색에 가까운 화소일수록 칸 색/밝기 평균에서 덜 반영해 번짐을 걷어 낸다.
    let weight = |i: usize| -> f32 {
        match key {
            Some(k) => (((dist(rgb[i], k) - o.bg_tol) / (0.5 - o.bg_tol).max(0.05)).clamp(0.02, 1.0)).powi(2),
            None => 1.0,
        }
    };
    // 피사체 모드에서는 어두운 선을 빈칸으로 비운다. 피사체 자체가 어두우면(검은 옷, 실루엣) 하지 않는다.
    let ink_on = has_bg && !o.invert && {
        let total = fg.iter().filter(|&&f| f).count();
        let dark = (0..n).filter(|&i| fg[i] && lum[i] < o.ink_thr).count();
        dark * 5 < total * 2
    };
    let shade_k = o.shade.unwrap_or(if has_bg { 1.0 } else { 0.0 }).clamp(0.0, 1.0);
    let shade = if shade_k > 0.0 && has_bg { Some(volume_shade(&fg, w, h)) } else { None };

    // 윤곽 강조용 기울기 (배경은 밝기 0 으로 보고 계산해 실루엣도 윤곽이 된다)
    let grads = if o.edges {
        let le: Vec<f32> = (0..n).map(|i| if fg[i] { tone(lum[i], o) } else { 0.0 }).collect();
        let at = |x: isize, y: isize| {
            let x = x.clamp(0, w as isize - 1) as usize;
            let y = y.clamp(0, h as isize - 1) as usize;
            le[y * w + x]
        };
        let mut g = vec![(0f32, 0f32); n];
        for y in 0..h as isize {
            for x in 0..w as isize {
                let gx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1))
                    - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
                let gy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1))
                    - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
                // 화면 y 는 아래로 증가하므로 위쪽 기준 각도로 뒤집는다
                g[y as usize * w + x as usize] = (gx / 8.0, -gy / 8.0);
            }
        }
        Some(g)
    } else {
        None
    };

    let levels = o.ramp.len() - 1;
    let per = (SX * SY) as f32;
    let cells = rows * cols;
    // 칸마다: (밝기, 면이 덮은 비율, 음영)
    let mut cell_l: Vec<(f32, f32, f32)> = Vec::with_capacity(cells);
    let mut edge_ch: Vec<Option<char>> = Vec::with_capacity(cells);
    let mut blanks = Vec::with_capacity(cells);
    // 선 칸: 선이 덮은 비율 (선 칸이 아니면 0)
    let mut inks: Vec<f32> = Vec::with_capacity(cells);
    let mut colors = Vec::with_capacity(cells);
    for cy in 0..rows {
        for cx in 0..cols {
            let (mut cnt, mut wsum, mut csum, mut lsum) = (0f32, 0f32, [0f32; 3], 0f32);
            let (mut dcnt, mut dsum) = (0f32, [0f32; 3]);
            let (mut jxx, mut jyy, mut jxy) = (0f32, 0f32, 0f32);
            for sy in 0..SY {
                let row = (cy * SY + sy) * w + cx * SX;
                for sx in 0..SX {
                    let i = row + sx;
                    if let Some(g) = &grads {
                        let (gx, gy) = g[i];
                        jxx += gx * gx;
                        jyy += gy * gy;
                        jxy += gx * gy;
                    }
                    // 면: 피사체에 속하고 선이 아닌 화소
                    if fg[i] && ink_on && lum[i] < o.ink_thr {
                        dcnt += 1.0;
                        dsum[0] += rgb[i][0];
                        dsum[1] += rgb[i][1];
                        dsum[2] += rgb[i][2];
                    } else if fg[i] {
                        let wt = weight(i);
                        cnt += 1.0;
                        wsum += wt;
                        csum[0] += rgb[i][0] * wt;
                        csum[1] += rgb[i][1] * wt;
                        csum[2] += rgb[i][2] * wt;
                        lsum += lum[i] * wt;
                    }
                }
            }
            let coverage = cnt / per;
            let safe = wsum.max(1e-6);
            let color = [csum[0] / safe, csum[1] / safe, csum[2] / safe];
            // 선이나 배경이 칸의 1/3 넘게 차지하면 비운다: 외곽선과 눈/입이 빈칸으로 드러난다
            let blank = has_bg && coverage < if ink_on { INK_FULL } else { 0.3 };
            // 선을 그리는 방식이면 선이 걸친 칸은 빈칸 대신 선 색 글자가 된다
            let ink = if blank && !o.ink_gap && dcnt / per >= 0.3 { dcnt / per } else { 0.0 };
            let blank = blank && ink == 0.0;
            inks.push(ink);
            blanks.push(blank);
            // 음영 격자에서 이 칸은 가로 1점, 세로 SY/SX 점이다
            let s = match &shade {
                Some((sv, sw, _)) => {
                    let k = SY / SX;
                    let sum: f32 = (0..k).map(|j| sv[(cy * k + j) * sw + cx]).sum();
                    1.0 - shade_k * (1.0 - sum / k as f32)
                }
                None => 1.0,
            };
            cell_l.push((lsum / safe, coverage, s));

            let mut ech = None;
            if grads.is_some() && !blank {
                let (jxx, jyy, jxy) = (jxx / per, jyy / per, jxy / per);
                let energy = jxx + jyy;
                let diff = ((jxx - jyy).powi(2) + 4.0 * jxy * jxy).sqrt();
                let strength = energy.max(0.0).sqrt();
                let coherence = diff / energy.max(1e-9);
                if strength > o.edge_thr && coherence > 0.6 {
                    let theta = 0.5 * (2.0 * jxy).atan2(jxx - jyy).to_degrees(); // 기울기 방향
                    let dir = (theta + 90.0).rem_euclid(180.0); // 경계선 방향
                    // 글자 칸이 1:2 라서 '/' 는 약 63도로 보인다
                    let gi = if dir < 30.0 { 0 } else if dir < 75.0 { 1 } else if dir < 105.0 { 2 } else if dir < 150.0 { 3 } else { 0 };
                    ech = Some(EDGE_GLYPHS[gi]);
                }
            }
            edge_ch.push(ech);

            // 밝기는 글자가, 색상은 전경색이 맡는다: 전경색 밝기를 끌어올려 어두운 칸도 색이 보이게.
            // 음영진 쪽은 색도 조금 가라앉혀 덩어리감을 더한다.
            // 장면 모드는 화면이 글자로 꽉 차므로 명암을 색이 맡는다: 바닥을 낮추고 채도를 조금 올린다.
            let floor = if has_bg { o.color_floor } else { o.color_floor * 0.3 };
            let v = color[0].max(color[1]).max(color[2]);
            let fgc = if v > 1e-4 {
                let k = (floor + (1.0 - floor) * v) / v * (0.72 + 0.28 * s);
                let mut c = [color[0] * k, color[1] * k, color[2] * k];
                if !has_bg {
                    let m = (c[0] + c[1] + c[2]) / 3.0;
                    c = c.map(|x| m + (x - m) * 1.25);
                }
                c
            } else {
                [floor; 3]
            };
            let fgc = if ink > 0.0 {
                // 선 색은 어두워서 그대로는 안 보인다: 색조는 두고 밝기만 끌어올린다
                let c = dsum.map(|x| x / dcnt);
                let m = c[0].max(c[1]).max(c[2]).max(0.04);
                c.map(|x| x / m * 0.55 + 0.04)
            } else {
                fgc
            };
            colors.push(fgc.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8));
        }
    }

    // 평활화: 칸 밝기를 화면 안의 순위(누적 분포)로 펴서 밝은 장면도 램프 전체를 쓰게 한다.
    let equalize = o.equalize.unwrap_or(if has_bg { 0.0 } else { 0.7 });
    let eq_rank: Option<Vec<f32>> = if equalize > 0.0 {
        let mut vals: Vec<f32> = cell_l.iter().zip(&blanks).filter(|(_, b)| !**b).map(|(l, _)| l.0).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if vals.len() > 1 {
            let n = vals.len() as f32;
            Some(cell_l.iter().map(|l| {
                // 같은 값이 여러 개면 가운데 순위를 쓴다
                let lo = vals.partition_point(|v| *v < l.0) as f32;
                let hi = vals.partition_point(|v| *v <= l.0) as f32;
                ((lo + hi) * 0.5 / n).clamp(0.0, 1.0)
            }).collect())
        } else {
            None
        }
    } else {
        None
    };

    // 램프의 옅은 앞부분(빈칸, 점)은 쓰지 않는다: 면은 그늘진 곳도 글자로 꽉 채운다
    // 장면 모드도 같은 글자 구간을 쓰되, 거의 검은 칸(레터박스, 밤하늘)만 비운다
    let lo_idx = (((levels as f32) * 0.25).ceil() as usize).min(levels);
    let mut chars = Vec::with_capacity(cells);
    for i in 0..cells {
        if blanks[i] {
            chars.push(' ');
            continue;
        }
        if inks[i] > 0.0 {
            // 굵게 덮인 칸(눈동자, 굵은 선)은 가장 진한 글자, 살짝 걸친 칸은 한 단계 옅게
            chars.push(if inks[i] >= 0.7 { o.ramp[levels] } else { INK_THIN });
            continue;
        }
        if let Some(c) = edge_ch[i] {
            chars.push(c);
            continue;
        }
        let (raw, coverage, s) = cell_l[i];
        if !has_bg && tone(raw, o) < 0.06 {
            chars.push(' ');
            continue;
        }
        let base = match &eq_rank {
            Some(r) => raw * (1.0 - equalize) + r[i] * equalize,
            None => raw,
        };
        let mut l = tone(base, o) * s;
        if has_bg {
            // 선이나 실루엣에 걸친 칸은 한두 단계 옅어져 경계가 부드럽게 끊긴다
            let full = if ink_on { INK_FULL } else { 0.3 };
            l *= 0.62 + 0.38 * ((coverage - full) / (0.95 - full)).clamp(0.0, 1.0);
        }
        let idx = lo_idx + (l * (levels - lo_idx) as f32).round() as usize;
        chars.push(o.ramp[idx.min(levels)]);
    }
    if has_bg {
        // 이웃이 거의 없는 외톨이 글자(선 사이에 낀 조각, 잘린 배경 부스러기)를 지운다
        let lone: Vec<usize> = (0..cells)
            .filter(|&i| {
                if chars[i] == ' ' {
                    return false;
                }
                let (cx, cy) = (i % cols, i / cols);
                let mut near = 0;
                for yy in cy.saturating_sub(1)..=(cy + 1).min(rows - 1) {
                    for xx in cx.saturating_sub(1)..=(cx + 1).min(cols - 1) {
                        if chars[yy * cols + xx] != ' ' {
                            near += 1;
                        }
                    }
                }
                near <= 2 // 자기 자신 포함
            })
            .collect();
        for i in lone {
            chars[i] = ' ';
        }
    }
    Grid { cols, rows, chars, colors, bold: o.bold }
}

/// 색 없는 텍스트 (줄 끝 공백 제거). --snapshot 과 테스트 비교용.
pub fn to_text(g: &Grid) -> String {
    let mut out = String::with_capacity(g.rows * (g.cols + 1));
    for r in 0..g.rows {
        let line: String = g.chars[r * g.cols..(r + 1) * g.cols].iter().collect();
        out.push_str(line.trim_end());
        if r + 1 < g.rows {
            out.push('\n');
        }
    }
    out
}

/// 터미널 문자열로 인코딩한다. 색이 바뀔 때만 색 코드를 내보내고,
/// 빈칸은 색이 상관없으니 앞 칸 색을 그대로 이어 쓴다.
pub fn to_ansi(g: &Grid, color: bool, out: &mut String) {
    let mut cur: Option<u32> = None;
    if color && g.bold {
        out.push_str("\x1b[1m");
    }
    for r in 0..g.rows {
        for c in 0..g.cols {
            let i = r * g.cols + c;
            let ch = g.chars[i];
            if color && ch != ' ' {
                let [cr, cg, cb] = g.colors[i].map(|v| v & !3); // 4 단위로 묶어 색 코드 수를 줄인다
                let packed = (cr as u32) << 16 | (cg as u32) << 8 | cb as u32;
                if cur != Some(packed) {
                    let _ = write!(out, "\x1b[38;2;{cr};{cg};{cb}m");
                    cur = Some(packed);
                }
            }
            out.push(ch);
        }
        if r + 1 < g.rows {
            out.push('\n');
        }
    }
    if color {
        out.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, rgba: [u8; 4]) -> Vec<u8> {
        (0..w * h).flat_map(|_| rgba).collect()
    }

    /// 배경색 위에 검은 테두리(두께 t)를 두른 사각형을 그린다. 안쪽은 fill.
    fn boxed(w: usize, h: usize, bg: impl Fn(usize, usize) -> [u8; 3], fill: [u8; 3], t: usize) -> Vec<u8> {
        let mut f = Vec::with_capacity(w * h * 4);
        let (x0, x1, y0, y1) = (w / 4, w * 3 / 4, h / 4, h * 3 / 4);
        for y in 0..h {
            for x in 0..w {
                let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
                let line = inside && (x < x0 + t || x >= x1 - t || y < y0 + t || y >= y1 - t);
                let c = if line { [20, 15, 10] } else if inside { fill } else { bg(x, y) };
                f.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        f
    }

    #[test]
    fn white_on_black_is_dense() {
        let o = Options { bg: BgMode::None, equalize: Some(0.0), ..Default::default() };
        let g = render(&solid(16, 16, [255, 255, 255, 255]), 16, 16, &o);
        assert_eq!((g.cols, g.rows), (4, 2));
        assert!(g.chars.iter().all(|&c| c == '@'));
        let g = render(&solid(16, 16, [0, 0, 0, 255]), 16, 16, &o);
        assert!(g.chars.iter().all(|&c| c == ' '));
    }

    #[test]
    fn dark_colors_keep_hue() {
        let o = Options { bg: BgMode::None, ..Default::default() };
        let g = render(&solid(SX, SY, [40, 0, 0, 255]), SX, SY, &o);
        let [r, gg, b] = g.colors[0];
        assert!(r > 50 && gg == 0 && b == 0, "{:?}", g.colors[0]);
    }

    #[test]
    fn border_flood_keeps_enclosed_interior() {
        // 흰 배경 위에 검은 테두리 사각형, 안쪽도 흰색. 안쪽은 배경으로 지워지면 안 되고, 테두리는 선 글자가 된다.
        let (w, h) = (160, 160);
        let f = boxed(w, h, |_, _| [255, 255, 255], [255, 255, 255], 8);
        let g = render(&f, w, h, &Options::default());
        let at = |cx: usize, cy: usize| g.chars[cy * g.cols + cx];
        assert_eq!(at(0, 0), ' ');
        assert_eq!(at(10, 5), '@'); // 테두리 (x 40..48, y 40..48) 는 선 색 글자
        assert!(at(20, 10).is_ascii_alphanumeric() || "&#$@".contains(at(20, 10)), "{:?}", at(20, 10));
    }

    #[test]
    fn outline_cuts_subject_out_of_scenery() {
        // 위는 하늘색, 아래는 연두색인 배경(단색 가장자리가 아님) 위의 외곽선 캐릭터
        let (w, h) = (160, 160);
        let f = boxed(w, h, |_, y| if y < 100 { [150, 210, 240] } else { [200, 230, 170] }, [255, 240, 190], 6);
        let g = render(&f, w, h, &Options::default());
        let at = |cx: usize, cy: usize| g.chars[cy * g.cols + cx];
        assert_eq!(at(1, 1), ' ');
        assert_eq!(at(1, g.rows - 2), ' ');
        assert_ne!(at(20, 10), ' ');
        // 사진처럼 외곽선이 없으면 장면 그대로 채운다
        let plain: Vec<u8> = (0..w * h).flat_map(|i| { let v = (i % w * 255 / w) as u8; [v, 255 - v, 128, 255] }).collect();
        let g = render(&plain, w, h, &Options::default());
        assert_eq!(g.nonblank(), g.cols * g.rows);
    }

    #[test]
    fn shade_is_brighter_toward_the_light() {
        // 투명 배경 위의 흰 원: 왼쪽 위가 오른쪽 아래보다 진한 글자여야 한다
        let (w, h) = (160usize, 160usize);
        let f: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let (x, y) = ((i % w) as f32 - 80.0, (i / w) as f32 - 80.0);
                if x * x + y * y < 70.0 * 70.0 { [255u8; 4] } else { [0u8; 4] }
            })
            .collect();
        let o = Options::default();
        let g = render(&f, w, h, &o);
        let rank = |cx: usize, cy: usize| o.ramp.iter().position(|&c| c == g.chars[cy * g.cols + cx]).unwrap();
        assert!(rank(12, 5) > rank(28, 15), "{} {}", rank(12, 5), rank(28, 15));
        assert_eq!(g.chars[0], ' ');
    }

    #[test]
    fn edges_follow_direction() {
        // 왼쪽 검정, 오른쪽 흰색인 세로 경계 -> '|'
        let (w, h) = (16, 16);
        let f: Vec<u8> = (0..w * h).flat_map(|i| if i % w >= 8 { [255; 4] } else { [0, 0, 0, 255] }).collect();
        let o = Options { bg: BgMode::None, edges: true, ..Default::default() };
        let g = render(&f, w, h, &o);
        assert_eq!(g.chars[g.cols + 1], '|');
        assert_eq!(g.chars[g.cols + 2], '|');
    }

    #[test]
    fn ansi_only_emits_color_changes() {
        let o = Options { bg: BgMode::None, ..Default::default() };
        let g = render(&solid(40, 16, [200, 100, 50, 255]), 40, 16, &o);
        let mut s = String::new();
        to_ansi(&g, true, &mut s);
        assert_eq!(s.matches("\x1b[38;2;").count(), 1);
    }
}
