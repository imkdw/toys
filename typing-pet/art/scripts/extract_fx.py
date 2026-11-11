"""장면 소품(콧방울, Zzz 등)을 생성 이미지에서 잘라 work/fx/<이름>.png 로 저장한다.

picks.json 의 "_fx" 항목: {"<이름>": {"src": "<소품이 있는 raw>", "ref": "<소품만 없는 raw>"}}
두 그림을 크로마키로 배경을 빼고 base 에 맞춘 뒤, src 에서 ref 와 달라진 덩어리만 남긴다.
slice.py 가 work/fx/*.png 를 fx/<이름> 레이어로 그대로 내보낸다.
소품 위치(anchor 를 잡을 때 참고)로 각 소품의 경계 상자를 출력한다.

사용법: extract_fx.py [--name usagi] [--thr 60]
"""

import argparse
import json
import tempfile
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import align
import chroma
from slice import shift_diff

ROOT = Path(__file__).resolve().parents[2]


def prep(raw: Path, base: np.ndarray, tmp: Path) -> np.ndarray:
    chroma.run(raw, tmp)
    var = align.load(tmp / raw.name)
    M, _, _ = align.estimate(base, var)
    return align.warp(var, M).astype(np.float32)


def flat_rgb(rgba: np.ndarray) -> np.ndarray:
    """투명한 곳은 크로마 초록으로 채운 RGB (빈 배경과 새로 생긴 그림이 구분되게)."""
    a = rgba[..., 3:4] / 255
    return rgba[..., :3] * a + np.array([0, 255, 0], np.float32) * (1 - a)


def extract(src: np.ndarray, ref: np.ndarray, thr: float, shift: int = 4) -> np.ndarray:
    # 정렬 오차로 생기는 윤곽선 어긋남은 shift px 안의 가장 비슷한 색과 비교해서 무시한다
    m = (shift_diff(flat_rgb(src), flat_rgb(ref), shift) > thr).astype(np.uint8)
    n, lab, st, _ = cv2.connectedComponentsWithStats(m)
    keep = np.zeros_like(m)
    for i in range(1, n):
        # 책상 가장자리처럼 얇고 긴 띠는 정렬 오차라서 버린다
        if st[i, cv2.CC_STAT_AREA] >= 40 and min(st[i, cv2.CC_STAT_WIDTH], st[i, cv2.CC_STAT_HEIGHT]) >= 10:
            keep[lab == i] = 1
    # 가까운 조각끼리 묶어 볼록 껍질로 메운다. 콧방울처럼 윤곽이 끊기거나 속이 얼굴과 비슷한 색이어도 통째로 잡힌다.
    # src 가 투명한 곳(빈 배경)은 알파가 0 이라 껍질에 들어가도 보이지 않는다
    groups = cv2.dilate(keep, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (25, 25)))
    n, lab = cv2.connectedComponents(groups)
    filled = np.zeros_like(keep)
    for i in range(1, n):
        pts = cv2.findNonZero(((lab == i) & (keep == 1)).astype(np.uint8))
        if pts is not None:
            cv2.fillConvexPoly(filled, cv2.convexHull(pts), 1)
    a = cv2.GaussianBlur(cv2.dilate(filled, np.ones((5, 5), np.uint8)).astype(np.float32), (5, 5), 1.2)
    out = src.copy()
    out[..., 3] = src[..., 3] * np.clip(a, 0, 1)
    return np.clip(out, 0, 255).astype(np.uint8)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", default="usagi")
    ap.add_argument("--thr", type=float, default=60)
    a = ap.parse_args()
    art = ROOT / "art" / a.name
    fx = json.loads((art / "picks.json").read_text()).get("_fx", {})
    out_dir = art / "work" / "fx"
    if out_dir.is_dir():
        for f in out_dir.glob("*.png"):
            f.unlink()
    if not fx:
        return
    out_dir.mkdir(parents=True, exist_ok=True)
    base = align.load(art / "work" / "base.png")
    with tempfile.TemporaryDirectory() as t:
        for name, spec in fx.items():
            tmp = Path(t) / name
            tmp.mkdir()
            src = prep(art / "raw" / f"{spec['src']}.png", base, tmp)
            ref = prep(art / "raw" / f"{spec['ref']}.png", base, tmp)
            rgba = extract(src, ref, a.thr)
            Image.fromarray(rgba, "RGBA").save(out_dir / f"{name}.png")
            ys, xs = np.nonzero(rgba[..., 3] > 128)
            box = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())] if len(xs) else None
            print(f"fx/{name}: 경계 상자 {box}")


if __name__ == "__main__":
    main()
