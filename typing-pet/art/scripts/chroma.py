"""초록 크로마키 배경을 투명으로 만든다 (PLAN 3.5-2).

- RGB 거리 대신 "초록이 얼마나 우세한가 + 색상(hue)" 으로 판정한다. 노란 소품(R, G 둘 다 높음)은 지워지지 않는다.
- 반투명 가장자리는 배경색을 빼서 원래 색을 되살리고(unmix), 남은 초록 번짐을 제거한다(despill).
- 마지막으로 알파를 1px 침식한다.

사용법: chroma.py <입력 png...> -o <출력 폴더>
"""

import argparse
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import cv2
import numpy as np
from PIL import Image


def key(rgb: np.ndarray) -> np.ndarray:
    """RGB uint8 (H, W, 3) -> RGBA uint8 (straight alpha)."""
    f = rgb.astype(np.float32)
    r, g, b = f[..., 0], f[..., 1], f[..., 2]
    # 배경색: 테두리 픽셀의 중앙값
    border = np.concatenate([f[:8].reshape(-1, 3), f[-8:].reshape(-1, 3), f[:, :8].reshape(-1, 3), f[:, -8:].reshape(-1, 3)])
    bg = np.median(border, axis=0)
    # 초록 우세도. 배경에서 최대, 갈색 외곽선/크림/노랑/흰색에서는 0 이하
    dom = g - np.maximum(r, b)
    bg_dom = max(bg[1] - max(bg[0], bg[2]), 1.0)
    hsv = cv2.cvtColor(rgb, cv2.COLOR_RGB2HSV_FULL).astype(np.float32)
    hue = hsv[..., 0] * 360.0 / 256.0
    greenish = (hue > 70) & (hue < 170)
    lo, hi = 0.18 * bg_dom, 0.85 * bg_dom
    t = np.clip((dom - lo) / (hi - lo), 0.0, 1.0)  # 1 = 배경
    t = np.where(greenish, t, 0.0)
    alpha = 1.0 - t
    # unmix: obs = a*C + (1-a)*bg  ->  C = (obs - (1-a)*bg) / a
    a3 = np.maximum(alpha, 1e-3)[..., None]
    col = (f - (1.0 - alpha)[..., None] * bg) / a3
    col = np.clip(col, 0, 255)
    # despill: 반투명/가장자리에서 G 가 R, B 의 최대값을 넘지 않게
    edge = alpha < 0.999
    cr, cg, cb = col[..., 0], col[..., 1], col[..., 2]
    cg = np.where(edge, np.minimum(cg, np.maximum(cr, cb)), cg)
    col = np.stack([cr, cg, cb], axis=-1)
    # 1px 침식
    a8 = (alpha * 255).astype(np.uint8)
    a8 = cv2.erode(a8, np.ones((3, 3), np.uint8))
    out = np.dstack([col.astype(np.uint8), a8])
    out[a8 == 0, :3] = 0
    return out


def run(src: Path, out_dir: Path) -> str:
    rgb = np.array(Image.open(src).convert("RGB"))
    out = key(rgb)
    dst = out_dir / src.name
    Image.fromarray(out, "RGBA").save(dst)
    return f"{src.name}: 투명 {(out[..., 3] == 0).mean() * 100:.1f}%"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("inputs", nargs="+", type=Path)
    ap.add_argument("-o", "--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    with ProcessPoolExecutor() as ex:
        for line in ex.map(run, args.inputs, [args.out] * len(args.inputs)):
            print(line)


if __name__ == "__main__":
    main()
