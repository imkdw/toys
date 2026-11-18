//! 크기 조절 틀 (메뉴의 "크기 조절").
//!
//! 포토샵 자르기 틀처럼 캐릭터 정사각형에 테두리와 손잡이 8개를 그리고,
//! 손잡이를 끌면 반대쪽을 고정한 채 크기를 바꾼다. 캐릭터는 늘 정사각형이라 가로/세로 비율은 고정.

use crate::config::clamp_size;

/// 캐릭터 정사각형 (화면 pt): 좌상단 x, y 와 한 변 s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub s: f64,
}

/// 손잡이 방향. x/y 가 -1 이면 왼쪽/위, 1 이면 오른쪽/아래, 0 이면 가운데. (0, 0) 은 틀 안 (옮기기).
pub type Handle = (i8, i8);

/// 손잡이를 잡을 수 있는 거리 (pt). 작은 캐릭터에서는 한 변의 1/4 까지만.
pub fn grab_zone(s: f64) -> f64 {
    14.0f64.min(s / 4.0)
}

/// 정사각형 안 위치 (pt, 좌상단 기준)에 있는 손잡이. 정사각형 밖이면 None.
pub fn handle_at(u: f64, v: f64, s: f64) -> Option<Handle> {
    if !(0.0..=s).contains(&u) || !(0.0..=s).contains(&v) {
        return None;
    }
    let z = grab_zone(s);
    let side = |t: f64| if t < z { -1 } else if t > s - z { 1 } else { 0 };
    Some((side(u), side(v)))
}

/// `start` 에서 손잡이 `h` 를 커서 `c` (화면 pt) 까지 끌었을 때의 정사각형.
/// 모서리는 맞은편 모서리를, 왼쪽/오른쪽 변은 맞은편 변과 아래쪽을 (책상에 앉은 채로),
/// 위/아래 변은 맞은편 변과 가로 가운데를 고정한다.
pub fn drag_rect(start: Rect, h: Handle, c: (f64, f64)) -> Rect {
    let Rect { x, y, s } = start;
    let ax = if h.0 > 0 { x } else { x + s };
    let ay = if h.1 > 0 { y } else { y + s };
    let dx = (c.0 - ax) * h.0 as f64;
    let dy = (c.1 - ay) * h.1 as f64;
    let ns = clamp_size(match h {
        (0, 0) => return start,
        (_, 0) => dx,
        (0, _) => dy,
        _ => dx.max(dy),
    });
    let nx = match h.0 {
        0 => x + (s - ns) / 2.0,
        1 => ax,
        _ => ax - ns,
    };
    let ny = match h.1 {
        0 => y + s - ns,
        1 => ay,
        _ => ay - ns,
    };
    Rect { x: nx, y: ny, s: ns }
}

/// 틀 그림 (premultiplied RGBA, `w` x `h` px). 캐릭터 정사각형은 위에서 `pad` px 아래부터. `scale` 은 화면 배율.
pub fn frame_image(w: u32, h: u32, pad: u32, scale: f32) -> Vec<u8> {
    let mut img = vec![0u8; (w as usize) * (h as usize) * 4];
    let (w, h, top) = (w as i32, h as i32, pad as i32);
    let side = w.min(h - top);
    if side < 8 {
        return img;
    }
    let mut rect = |x0: i32, y0: i32, x1: i32, y1: i32, c: [u8; 4]| {
        for y in y0.max(0)..y1.min(h) {
            for x in x0.max(0)..x1.min(w) {
                let i = ((y * w + x) * 4) as usize;
                img[i..i + 4].copy_from_slice(&c);
            }
        }
    };
    const FILL: [u8; 4] = [0, 0, 0, 22];
    const GRID: [u8; 4] = [110, 110, 110, 110];
    const DARK: [u8; 4] = [0, 0, 0, 150];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    let one = (scale.round() as i32).max(1);
    let line = ((1.5 * scale).round() as i32).max(1);
    let (x1, y1) = (side, top + side);
    // 살짝 어둡게 (틀 안 어디를 눌러도 창이 받도록 투명한 곳을 없앤다)
    rect(0, top, x1, y1, FILL);
    // 3등분 안내선
    for k in 1..3 {
        let gx = k * side / 3;
        rect(gx, top, gx + one, y1, GRID);
        let gy = top + k * side / 3;
        rect(0, gy, x1, gy + one, GRID);
    }
    // 테두리: 흰 선 + 안쪽 어두운 선 (밝은/어두운 바탕 모두에서 보이게)
    let ring = |rect: &mut dyn FnMut(i32, i32, i32, i32, [u8; 4]), t: i32, c: [u8; 4]| {
        rect(0, top, x1, top + t, c);
        rect(0, y1 - t, x1, y1, c);
        rect(0, top, t, y1, c);
        rect(x1 - t, top, x1, y1, c);
    };
    ring(&mut rect, line + one, DARK);
    ring(&mut rect, line, WHITE);
    // 손잡이 8개
    let hs = handle_px(side, scale);
    for (i, hx) in [0, (side - hs) / 2, side - hs].into_iter().enumerate() {
        for (j, hy) in [top, top + (side - hs) / 2, y1 - hs].into_iter().enumerate() {
            if i == 1 && j == 1 {
                continue;
            }
            rect(hx, hy, hx + hs, hy + hs, DARK);
            rect(hx + one, hy + one, hx + hs - one, hy + hs - one, WHITE);
        }
    }
    img
}

/// premultiplied 그림 `src` (`sw` x `sh`) 를 `dst` (`dw` x `dh`) 의 (x, y) 에 얹는다 (source-over).
pub fn paste(dst: &mut [u8], dw: u32, dh: u32, src: &[u8], sw: u32, sh: u32, x: i32, y: i32) {
    for sy in 0..sh as i32 {
        let ty = y + sy;
        if ty < 0 || ty >= dh as i32 {
            continue;
        }
        for sx in 0..sw as i32 {
            let tx = x + sx;
            if tx < 0 || tx >= dw as i32 {
                continue;
            }
            let si = ((sy * sw as i32 + sx) * 4) as usize;
            let di = ((ty * dw as i32 + tx) * 4) as usize;
            let a = src[si + 3] as u32;
            for k in 0..4 {
                dst[di + k] = (src[si + k] as u32 + dst[di + k] as u32 * (255 - a) / 255).min(255) as u8;
            }
        }
    }
}

/// 손잡이 한 변 (px). 틀과 안내문 자리를 맞추려고 따로 둔다.
pub fn handle_px(side: i32, scale: f32) -> i32 {
    ((10.0 * scale).round() as i32).min(side / 5).max(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect { x: 100.0, y: 100.0, s: 200.0 };

    #[test]
    fn handles() {
        assert_eq!(handle_at(2.0, 2.0, 200.0), Some((-1, -1)));
        assert_eq!(handle_at(198.0, 100.0, 200.0), Some((1, 0)));
        assert_eq!(handle_at(100.0, 199.0, 200.0), Some((0, 1)));
        assert_eq!(handle_at(100.0, 100.0, 200.0), Some((0, 0)));
        assert_eq!(handle_at(100.0, -5.0, 200.0), None);
    }

    #[test]
    fn corner_keeps_opposite_corner() {
        let r = drag_rect(R, (1, 1), (400.0, 380.0));
        assert_eq!(r, Rect { x: 100.0, y: 100.0, s: 300.0 });
        let r = drag_rect(R, (-1, -1), (50.0, 80.0));
        assert_eq!((r.x + r.s, r.y + r.s, r.s), (300.0, 300.0, 250.0));
    }

    #[test]
    fn side_keeps_bottom() {
        let r = drag_rect(R, (1, 0), (340.0, 0.0));
        assert_eq!(r, Rect { x: 100.0, y: 60.0, s: 240.0 });
        let r = drag_rect(R, (0, -1), (0.0, 140.0));
        assert_eq!(r, Rect { x: 120.0, y: 140.0, s: 160.0 });
    }

    #[test]
    fn clamps_size() {
        let r = drag_rect(R, (1, 1), (90.0, 90.0));
        assert_eq!(r.s, crate::config::MIN_SIZE);
        assert_eq!((r.x, r.y), (100.0, 100.0));
        let r = drag_rect(R, (-1, 1), (-5000.0, 0.0));
        assert_eq!(r.s, crate::config::MAX_SIZE);
        assert_eq!(r.x + r.s, 300.0);
    }

    #[test]
    fn frame_has_no_holes_inside() {
        let (w, pad) = (120u32, 30u32);
        let img = frame_image(w, w + pad, pad, 2.0);
        // 띠는 투명, 정사각형 안은 전부 불투명한 픽셀이 조금이라도 있다
        assert!(img[..(w * pad * 4) as usize].iter().all(|&b| b == 0));
        assert!(img[(w * pad * 4) as usize..].chunks_exact(4).all(|p| p[3] > 0));
    }
}
