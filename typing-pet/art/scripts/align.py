"""변형 프레임을 base 에 맞춘다 (PLAN 3.5-3).

생성 모델은 위치뿐 아니라 크기도 미세하게 바꾸므로 이동 + 확대축소 + 회전(similarity) 변환을 찾는다.
SIFT 특징점을 RANSAC 으로 맞추므로, 바뀐 부분(앞발, 얼굴)은 이상치로 빠지고 책상/키보드/소품 같은 안 바뀐 부분이 기준이 된다.

사용법: align.py --base <base RGBA> <변형 RGBA...> -o <출력 폴더>
"""

import argparse
import json
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import cv2
import numpy as np
from PIL import Image


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA"))


def gray_on_white(rgba: np.ndarray) -> np.ndarray:
    a = rgba[..., 3:4].astype(np.float32) / 255
    rgb = rgba[..., :3].astype(np.float32) * a + 255 * (1 - a)
    return cv2.cvtColor(rgb.astype(np.uint8), cv2.COLOR_RGB2GRAY)


def estimate(base: np.ndarray, var: np.ndarray):
    sift = cv2.SIFT_create(nfeatures=6000)
    kb, db = sift.detectAndCompute(gray_on_white(base), None)
    kv, dv = sift.detectAndCompute(gray_on_white(var), None)
    matches = cv2.BFMatcher(cv2.NORM_L2).knnMatch(dv, db, k=2)
    good = [m for m, n in (p for p in matches if len(p) == 2) if m.distance < 0.75 * n.distance]
    src = np.float32([kv[m.queryIdx].pt for m in good])
    dst = np.float32([kb[m.trainIdx].pt for m in good])
    M, inl = cv2.estimateAffinePartial2D(src, dst, method=cv2.RANSAC, ransacReprojThreshold=1.5, maxIters=5000, confidence=0.999)
    if M is None:
        return np.float32([[1, 0, 0], [0, 1, 0]]), 0, len(good)
    return M, int(inl.sum()), len(good)


def warp(rgba: np.ndarray, M: np.ndarray) -> np.ndarray:
    """premultiplied 상태로 변환해서 가장자리 색 번짐을 막는다."""
    f = rgba.astype(np.float32)
    a = f[..., 3:4] / 255
    pre = np.dstack([f[..., :3] * a, f[..., 3]])
    h, w = rgba.shape[:2]
    out = cv2.warpAffine(pre, M, (w, h), flags=cv2.INTER_CUBIC, borderMode=cv2.BORDER_CONSTANT, borderValue=(0, 0, 0, 0))
    out = np.clip(out, 0, 255)
    a2 = out[..., 3:4] / 255
    rgb = np.where(a2 > 1e-3, out[..., :3] / np.maximum(a2, 1e-3), 0)
    return np.dstack([np.clip(rgb, 0, 255), out[..., 3]]).astype(np.uint8)


def run(args):
    base_p, var_p, out_dir = args
    base, var = load(base_p), load(var_p)
    M, inliers, good = estimate(base, var)
    s = float(np.sqrt(M[0, 0] ** 2 + M[1, 0] ** 2))
    rot = float(np.degrees(np.arctan2(M[1, 0], M[0, 0])))
    out = warp(var, M)
    Image.fromarray(out, "RGBA").save(out_dir / var_p.name)
    info = {"file": var_p.name, "scale": round(s, 5), "rot_deg": round(rot, 4), "tx": round(float(M[0, 2]), 2), "ty": round(float(M[1, 2]), 2),
            "inliers": inliers, "matches": good}
    return info


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", type=Path, required=True)
    ap.add_argument("inputs", nargs="+", type=Path)
    ap.add_argument("-o", "--out", type=Path, required=True)
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    with ProcessPoolExecutor() as ex:
        infos = list(ex.map(run, [(a.base, p, a.out) for p in a.inputs]))
    for i in infos:
        print(json.dumps(i, ensure_ascii=False))
    (a.out / "align.json").write_text(json.dumps(infos, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
