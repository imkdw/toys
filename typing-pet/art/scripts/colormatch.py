"""변형 프레임 색을 base 에 맞춘다 (PLAN 3.5-4).

안 바뀐 영역(두 그림 모두 불투명하고 색 차이가 작은 곳)끼리 비교해 채널별 gain/bias 를 구하고 변형 전체에 적용한다.
안 하면 레이어 경계에서 책상 색이 패치처럼 티 난다.

사용법: colormatch.py --base <base RGBA> <정렬된 변형 RGBA...> -o <출력 폴더> [--mask <제외 마스크 png>]
"""

import argparse
import json
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
from PIL import Image


def fit(base: np.ndarray, var: np.ndarray, exclude: np.ndarray | None):
    both = (base[..., 3] == 255) & (var[..., 3] == 255)
    if exclude is not None:
        both &= exclude < 16
    b = base[..., :3].astype(np.float32)
    v = var[..., :3].astype(np.float32)
    sel = both & (np.abs(b - v).max(axis=-1) < 40)
    params = []
    for c in range(3):
        x, y = v[..., c][sel], b[..., c][sel]
        # 두 번 반복해서 큰 잔차를 버림
        g, o = 1.0, 0.0
        for _ in range(2):
            A = np.stack([x, np.ones_like(x)], axis=1)
            g, o = np.linalg.lstsq(A, y, rcond=None)[0]
            r = np.abs(g * x + o - y)
            keep = r < max(np.percentile(r, 90), 2.0)
            x, y = x[keep], y[keep]
        params.append((float(g), float(o)))
    return params, int(sel.sum())


def run(args):
    base_p, var_p, out_dir, mask_p = args
    base = np.array(Image.open(base_p).convert("RGBA"))
    var = np.array(Image.open(var_p).convert("RGBA"))
    excl = np.array(Image.open(mask_p).convert("L")) if mask_p else None
    params, n = fit(base, var, excl)
    out = var.astype(np.float32)
    before = np.abs(base[..., :3].astype(np.float32) - var[..., :3]).mean()
    for c, (g, o) in enumerate(params):
        out[..., c] = out[..., c] * g + o
    out = np.clip(out, 0, 255).astype(np.uint8)
    after = np.abs(base[..., :3].astype(np.float32) - out[..., :3]).mean()
    Image.fromarray(out, "RGBA").save(out_dir / var_p.name)
    return {"file": var_p.name, "pixels": n, "gain_bias": [[round(g, 4), round(o, 2)] for g, o in params],
            "mean_abs_diff": [round(float(before), 2), round(float(after), 2)]}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", type=Path, required=True)
    ap.add_argument("--mask", type=Path)
    ap.add_argument("inputs", nargs="+", type=Path)
    ap.add_argument("-o", "--out", type=Path, required=True)
    a = ap.parse_args()
    a.out.mkdir(parents=True, exist_ok=True)
    with ProcessPoolExecutor() as ex:
        infos = list(ex.map(run, [(a.base, p, a.out, a.mask) for p in a.inputs]))
    for i in infos:
        print(json.dumps(i, ensure_ascii=False))


if __name__ == "__main__":
    main()
