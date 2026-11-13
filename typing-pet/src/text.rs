//! 키 이름 말풍선(구름) 그림을 CoreText/CoreGraphics 로 만든다. 글꼴 대체(⌘ ⇧ 같은 기호)는 CoreText 가 알아서 한다.

use std::ffi::c_void;

use crate::render::SrcImage;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    w: f64,
    h: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGRect {
    o: CGPoint,
    s: CGSize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGAffineTransform {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

type Ref = *mut c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(r: *const c_void);
    fn CFStringCreateWithBytes(alloc: *const c_void, bytes: *const u8, len: isize, enc: u32, external: bool) -> Ref;
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        n: isize,
        key_cb: *const c_void,
        val_cb: *const c_void,
    ) -> Ref;
    fn CFAttributedStringCreate(alloc: *const c_void, s: Ref, attrs: Ref) -> Ref;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    static kCFBooleanTrue: *const c_void;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> Ref;
    fn CGColorSpaceRelease(cs: Ref);
    fn CGBitmapContextCreate(data: *mut c_void, w: usize, h: usize, bpc: usize, bpr: usize, cs: Ref, info: u32) -> Ref;
    fn CGContextRelease(c: Ref);
    fn CGContextSetRGBFillColor(c: Ref, r: f64, g: f64, b: f64, a: f64);
    fn CGContextSetRGBStrokeColor(c: Ref, r: f64, g: f64, b: f64, a: f64);
    fn CGContextSetLineWidth(c: Ref, w: f64);
    fn CGContextAddEllipseInRect(c: Ref, r: CGRect);
    fn CGContextAddPath(c: Ref, p: Ref);
    fn CGContextDrawPath(c: Ref, mode: i32);
    fn CGContextSetShadowWithColor(c: Ref, offset: CGSize, blur: f64, color: Ref);
    fn CGContextBeginTransparencyLayer(c: Ref, aux: *const c_void);
    fn CGContextEndTransparencyLayer(c: Ref);
    fn CGContextSetTextMatrix(c: Ref, t: CGAffineTransform);
    fn CGContextSetTextPosition(c: Ref, x: f64, y: f64);
    fn CGColorCreateGenericRGB(r: f64, g: f64, b: f64, a: f64) -> Ref;
    fn CGColorRelease(c: Ref);
    fn CGPathCreateWithRoundedRect(r: CGRect, cw: f64, ch: f64, t: *const c_void) -> Ref;
    fn CGPathRelease(p: Ref);
}

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontCreateUIFontForLanguage(ty: u32, size: f64, lang: *const c_void) -> Ref;
    fn CTLineCreateWithAttributedString(s: Ref) -> Ref;
    fn CTLineGetImageBounds(line: Ref, ctx: Ref) -> CGRect;
    fn CTLineDraw(line: Ref, ctx: Ref);
    static kCTFontAttributeName: *const c_void;
    static kCTForegroundColorFromContextAttributeName: *const c_void;
}

const UTF8: u32 = 0x0800_0100;
const UI_FONT_EMPHASIZED: u32 = 3;
const FILL: i32 = 0;
const FILL_STROKE: i32 = 3;
/// kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big: 메모리에 R, G, B, A 순서 (premultiplied).
const RGBA_PREMUL: u32 = 1 | (4 << 12);

/// 선 색 (캐릭터 외곽선처럼 짙은 갈색 회색).
const INK: (f64, f64, f64) = (0.29, 0.24, 0.25);

/// 글자 한 줄 (CTLine). 다 쓰면 놓는다.
struct Line(Ref);

impl Line {
    fn new(label: &str, size: f64) -> Option<Self> {
        unsafe {
            let font = CTFontCreateUIFontForLanguage(UI_FONT_EMPHASIZED, size, std::ptr::null());
            if font.is_null() {
                return None;
            }
            let s = CFStringCreateWithBytes(std::ptr::null(), label.as_ptr(), label.len() as isize, UTF8, false);
            let keys = [kCTFontAttributeName, kCTForegroundColorFromContextAttributeName];
            let vals = [font as *const c_void, kCFBooleanTrue];
            let attrs = CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                vals.as_ptr(),
                2,
                &kCFTypeDictionaryKeyCallBacks as *const u8 as *const c_void,
                &kCFTypeDictionaryValueCallBacks as *const u8 as *const c_void,
            );
            let astr = CFAttributedStringCreate(std::ptr::null(), s, attrs);
            let line = CTLineCreateWithAttributedString(astr);
            CFRelease(astr);
            CFRelease(attrs);
            CFRelease(s);
            CFRelease(font);
            (!line.is_null()).then_some(Self(line))
        }
    }
}

impl Drop for Line {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) };
    }
}

/// 비트맵 그리기 판. 픽셀은 `data` 에 남는다.
struct Canvas {
    ctx: Ref,
    data: Vec<u8>,
}

impl Canvas {
    fn new(w: usize, h: usize) -> Option<Self> {
        let mut data = vec![0u8; w * h * 4];
        unsafe {
            let cs = CGColorSpaceCreateDeviceRGB();
            let ctx = CGBitmapContextCreate(data.as_mut_ptr() as *mut c_void, w, h, 8, w * 4, cs, RGBA_PREMUL);
            CGColorSpaceRelease(cs);
            if ctx.is_null() {
                return None;
            }
            CGContextSetTextMatrix(ctx, CGAffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 });
            Some(Self { ctx, data })
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe { CGContextRelease(self.ctx) };
    }
}

/// 구름 모양: 아래는 납작한 알약, 위는 동글동글한 덩어리 (가운데가 가장 크고 높다).
/// 덩어리는 높이 기준 크기라 글자가 길어져도 찌그러지지 않고 개수만 늘어난다. CG 좌표 (원점 왼쪽 아래).
fn cloud_path(ctx: Ref, x0: f64, y0: f64, bw: f64, bh: f64) {
    unsafe {
        let base_h = bh * 0.56;
        let r = base_h * 0.5;
        let path = CGPathCreateWithRoundedRect(CGRect { o: CGPoint { x: x0, y: y0 }, s: CGSize { w: bw, h: base_h } }, r, r, std::ptr::null());
        CGContextAddPath(ctx, path);
        CGPathRelease(path);
        let n = ((bw / (bh * 0.5)).round() as usize).clamp(3, 7);
        let (l, rgt) = (x0 + bh * 0.4, x0 + bw - bh * 0.4);
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            // 가운데일수록 1. 가운데 하나만 도드라지게 제곱
            let mid = (1.0 - (t - 0.5).abs() * 2.0).powi(2);
            let rr = bh * (0.26 + 0.09 * mid);
            let cx = l + (rgt - l) * t;
            let cy = y0 + bh * (0.5 + 0.17 * mid);
            CGContextAddEllipseInRect(ctx, CGRect { o: CGPoint { x: cx - rr, y: cy - rr }, s: CGSize { w: rr * 2.0, h: rr * 2.0 } });
        }
    }
}

/// 키 이름이 적힌 구름 말풍선. 높이 `h` px, 너비는 글자 길이에 따라. premultiplied RGBA.
pub fn cloud(label: &str, h: u32) -> Option<SrcImage> {
    let h = h.max(8) as f64;
    let margin = (h * 0.13).ceil();
    let bh = h - margin * 2.0;
    // 기호 한 글자는 크게, 글자가 여럿이면(space, esc, F12) 조금 작게
    let chars = label.chars().count();
    let font = bh * if chars <= 1 { 0.44 } else { 0.36 };
    let line = Line::new(label, font)?;
    // 글자 크기를 재려면 판이 필요하다 (1x1 로 잰다)
    let probe = Canvas::new(1, 1)?;
    let tb = unsafe { CTLineGetImageBounds(line.0, probe.ctx) };
    drop(probe);
    let bw = (tb.s.w + bh * 0.95).max(bh * 1.35);
    let w = (bw + margin * 2.0).ceil() as usize;
    let hh = h as usize;
    let cv = Canvas::new(w, hh)?;
    let lw = (h * 0.035).max(1.0);
    let (x0, y0) = (margin, margin);
    unsafe {
        let ctx = cv.ctx;
        // 외곽선 + 그림자: 겹친 도형의 그림자가 한 번만 지도록 투명 레이어로 묶는다
        let shadow = CGColorCreateGenericRGB(0.0, 0.0, 0.0, 0.22);
        CGContextSetShadowWithColor(ctx, CGSize { w: 0.0, h: -h * 0.025 }, h * 0.07, shadow);
        CGContextBeginTransparencyLayer(ctx, std::ptr::null());
        CGContextSetRGBFillColor(ctx, INK.0, INK.1, INK.2, 1.0);
        CGContextSetRGBStrokeColor(ctx, INK.0, INK.1, INK.2, 1.0);
        CGContextSetLineWidth(ctx, lw * 2.0);
        cloud_path(ctx, x0, y0, bw, bh);
        CGContextDrawPath(ctx, FILL_STROKE);
        CGContextEndTransparencyLayer(ctx);
        CGContextSetShadowWithColor(ctx, CGSize::default(), 0.0, std::ptr::null_mut());
        CGColorRelease(shadow);
        // 안쪽 흰 구름 (도형끼리 겹친 선이 안 보이게 채우기만)
        CGContextSetRGBFillColor(ctx, 1.0, 0.995, 0.985, 1.0);
        cloud_path(ctx, x0, y0, bw, bh);
        CGContextDrawPath(ctx, FILL);
        // 양 볼 (연분홍)
        CGContextSetRGBFillColor(ctx, 1.0, 0.80, 0.84, 1.0);
        let (br, bry) = (bh * 0.085, bh * 0.05);
        for bx in [x0 + bh * 0.24, x0 + bw - bh * 0.24] {
            CGContextAddEllipseInRect(ctx, CGRect { o: CGPoint { x: bx - br, y: y0 + bh * 0.3 - bry }, s: CGSize { w: br * 2.0, h: bry * 2.0 } });
        }
        CGContextDrawPath(ctx, FILL);
        // 글자: 알약 부분과 덩어리 사이 가운데쯤
        let (cx, cy) = (x0 + bw / 2.0, y0 + bh * 0.42);
        CGContextSetRGBFillColor(ctx, INK.0, INK.1, INK.2, 1.0);
        CGContextSetTextPosition(ctx, cx - (tb.o.x + tb.s.w / 2.0), cy - (tb.o.y + tb.s.h / 2.0));
        CTLineDraw(line.0, ctx);
    }
    let mut cv = cv;
    let data = std::mem::take(&mut cv.data);
    drop(cv);
    Some(SrcImage { w: w as u32, h: hh as u32, data })
}
