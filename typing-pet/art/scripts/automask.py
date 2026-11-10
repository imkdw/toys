"""레이어 영역 마스크 초안을 base 와 변형의 차이로 자동 생성한다 (PLAN 3.2).

PLAN 은 마스크를 손으로 칠하게 되어 있지만, 먼저 차이 영역으로 초안을 만들어 art/masks/ 에 둔다.
손으로 고친 마스크가 이미 있으면 --force 없이는 덮어쓰지 않는다.

규칙:
- head, paw_l, paw_r 는 서로 겹치지 않는다. 머리 마스크에서 앞발 마스크 주변(페더링 폭 x 2)을 뺀다.
- 차이 영역에 여유(margin)를 더하고 구멍을 메운다. 페더링은 slice.py 가 한다.
- sunglasses 는 선글라스 변형의 차이 영역 중 어두운 픽셀만.
- 발끝 좌표(키 빛 위치)와 머리 기준점을 art/work/geometry.json 에 남긴다.

사용법: automask.py [--force]
"""

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "art/work"
MASKS = ROOT / "art/masks"

GROUPS = {
    "paw_l": ["paw_l/down"],
    "paw_r": ["paw_r/down", "paw_r/center", "paw_r/mouse", "paw_r/mouse_click"],
    "head": ["head/excited", "head/dizzy", "head/overheat", "head/sleepy", "head/sunglasses"],
}
MARGIN = 10
FEATHER = 6


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA")).astype(np.float32)


def premul(x: np.ndarray) -> np.ndarray:
    return np.dstack([x[..., :3] * x[..., 3:4] / 255, x[..., 3]])


def diff_mask(base: np.ndarray, var: np.ndarray, thr: float = 48) -> np.ndarray:
    d = np.abs(premul(base) - premul(var)).max(axis=-1)
    m = (d > thr).astype(np.uint8) * 255
    # 외곽선 재작화 같은 가는 선 잡음 제거 후 큰 덩어리만 남김
    m = cv2.morphologyEx(m, cv2.MORPH_OPEN, np.ones((5, 5), np.uint8))
    n, lab, stats, _ = cv2.connectedComponentsWithStats(m, 8)
    keep = np.zeros_like(m)
    for i in range(1, n):
        if stats[i, cv2.CC_STAT_AREA] >= 400:
            keep[lab == i] = 255
    return keep


def fill(m: np.ndarray, close: int = 25) -> np.ndarray:
    m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (close, close)))
    # 구멍 메우기: 바깥에서 flood fill 한 나머지
    h, w = m.shape
    ff = m.copy()
    cv2.floodFill(ff, np.zeros((h + 2, w + 2), np.uint8), (0, 0), 255)
    return m | cv2.bitwise_not(ff)


def dilate(m: np.ndarray, r: int) -> np.ndarray:
    return cv2.dilate(m, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2 * r + 1, 2 * r + 1)))


def tip(diff: np.ndarray) -> list[float]:
    """눌린 발의 끝: 차이 영역 중 가장 아래쪽 8% 픽셀의 평균."""
    ys, xs = np.nonzero(diff)
    if len(ys) == 0:
        return [0.0, 0.0]
    cut = np.percentile(ys, 92)
    sel = ys >= cut
    return [round(float(xs[sel].mean()), 1), round(float(ys[sel].mean()), 1)]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()
    picks = json.loads((ROOT / "art/picks.json").read_text())
    base = load(WORK / "base.png")
    MASKS.mkdir(parents=True, exist_ok=True)

    diffs: dict[str, np.ndarray] = {}
    for layers in GROUPS.values():
        for layer in layers:
            p = WORK / "matched" / f"{picks.get(layer, '')}.png"
            if layer in picks and p.exists():
                diffs[layer] = diff_mask(base, load(p))
            else:
                print(f"없음, 건너뜀: {p.name}")

    masks: dict[str, np.ndarray] = {}
    for g, layers in GROUPS.items():
        u = np.zeros(base.shape[:2], np.uint8)
        for layer in layers:
            if layer in diffs:
                u |= diffs[layer]
        masks[g] = dilate(fill(u), MARGIN)
    # 겹침 해소: 앞발끼리는 오른발 우선, 머리는 앞발 주변을 비움
    masks["paw_l"] &= cv2.bitwise_not(dilate(masks["paw_r"], 2 * FEATHER))
    paws = dilate(masks["paw_l"] | masks["paw_r"], 2 * FEATHER)
    masks["head"] &= cv2.bitwise_not(paws)
    # 선글라스: 차이 영역 중 어두운 픽셀
    sg = WORK / "matched" / f"{picks['head/sunglasses']}.png"
    if sg.exists() and "head/sunglasses" in diffs:
        v = load(sg)
        dark = (v[..., :3].max(axis=-1) < 70) & (v[..., 3] > 200)
        m = (dark & (dilate(diffs["head/sunglasses"], 4) > 0)).astype(np.uint8) * 255
        m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
        n, lab, stats, _ = cv2.connectedComponentsWithStats(m, 8)
        keep = np.zeros_like(m)
        for i in range(1, n):
            if stats[i, cv2.CC_STAT_AREA] >= 300:
                keep[lab == i] = 255
        masks["sunglasses"] = dilate(keep, 2)

    for g, m in masks.items():
        out = MASKS / f"{g}.png"
        if out.exists() and not a.force:
            print(f"이미 있음 (--force 로 덮어쓰기): {out.relative_to(ROOT)}")
            continue
        Image.fromarray(m, "L").save(out)
        print(f"{out.relative_to(ROOT)}: {(m > 0).mean() * 100:.1f}%")

    # 발끝 좌표, 머리 기준점
    geo = {"tips": {}, "head_anchor": None}
    for layer in ["paw_l/down", "paw_r/down", "paw_r/center", "paw_r/mouse_click"]:
        if layer in diffs:
            geo["tips"][layer] = tip(diffs[layer])
    head_alpha = (base[..., 3] > 128).astype(np.uint8)
    hm = np.array(Image.open(MASKS / "head.png").convert("L")) > 0
    ys, xs = np.nonzero(hm & (head_alpha > 0))
    geo["head_anchor"] = [round(float(xs.mean()), 1), round(float(ys.max()), 1)]
    (WORK / "geometry.json").write_text(json.dumps(geo, indent=1))
    print(json.dumps(geo))


if __name__ == "__main__":
    main()
