"""a/b 프레임 정합 (docs/MOTION_FIX_PLAN.md G3, docs/MOTION_QA.md T2).

a/b 를 따로 생성하면 움직이는 부위 밖(몸 윤곽, 책상)도 조금씩 달라서 교대할 때 몸 전체가 튄다.
a 를 기준으로 두 그림의 차이 덩어리(움직이는 부위)만 b 에서 가져와 a 위에 합성한다.
움직이는 부위 밖은 a 와 픽셀까지 같아진다.

picks.json 의 "_pair": {"full/<b 프레임>": "full/<a 프레임>"} 에 적힌 짝마다
(값을 {"from": "full/<a 프레임>", "keep_a": [[x0, y0, x1, y1], ..]} 로 쓰면 상자 안은 a 그대로: b 에 남은 떨어진 조각 지우기) characters/<이름>/layers 의 b 레이어를 고쳐 쓴다.
slice.py 가 레이어를 새로 쓰므로 그 다음에 돌린다 (build.sh).

사용법:
  pair_frames.py --name usagi                 # picks.json 의 _pair 를 적용
  pair_frames.py --a A.png --b B.png -o OUT   # 파일 하나 (확인용, OUT_cmp.png 에 a | b | 결과 | 마스크)
"""

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]


def flat(im: np.ndarray) -> np.ndarray:
    a = im[..., 3:4] / 255.0
    return im[..., :3] * a + 255 * (1 - a)


def moving_mask(a: np.ndarray, b: np.ndarray, min_area: int = 150, grow: int = 21, feather: float = 3.0) -> np.ndarray:
    """움직이는 부위 (0~1). 2px 이하로 어긋난 윤곽선 줄(가는 차이)은 빼고, 남은 덩어리를 넓혀 구멍을 메운다."""
    s = a.shape[0] / 1024
    d = (np.abs(flat(a) - flat(b)).max(axis=-1) > 40).astype(np.uint8)
    d = cv2.morphologyEx(d, cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
    n, lab, st, _ = cv2.connectedComponentsWithStats(d, 8)
    keep = np.isin(lab, [i for i in range(1, n) if st[i, cv2.CC_STAT_AREA] >= min_area * s * s])
    g = int(grow * s) | 1
    m = cv2.morphologyEx(keep.astype(np.uint8), cv2.MORPH_CLOSE, np.ones((g, g), np.uint8))
    m = cv2.dilate(m, np.ones((g, g), np.uint8))
    # 구멍 메우기 (덩어리 안쪽의 같은 픽셀도 b 에서)
    ff = np.pad(m, 1).copy()
    cv2.floodFill(ff, None, (0, 0), 1)
    m = m | (1 - ff[1:-1, 1:-1])
    m = cv2.GaussianBlur(m.astype(np.float32), (0, 0), feather * s)
    return np.clip(m * 2.0, 0.0, 1.0)


def pair(a: np.ndarray, b: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """(a 위에 b 의 움직이는 부위만 얹은 b, 마스크)."""
    m = moving_mask(a, b)[..., None]
    return a * (1 - m) + b * m, m[..., 0]


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA")).astype(np.float32)


def save(p: Path, im: np.ndarray) -> None:
    Image.fromarray(np.clip(im, 0, 255).astype(np.uint8)).save(p)


def compare_sheet(a: np.ndarray, b: np.ndarray, o: np.ndarray, m: np.ndarray, out: Path, s: int = 400) -> None:
    sh = Image.new("RGB", (s * 4, s), "white")
    for k, im in enumerate([a, b, o]):
        x = Image.fromarray(np.clip(im, 0, 255).astype(np.uint8))
        bg = Image.new("RGB", x.size, "white")
        bg.paste(x, mask=x)
        sh.paste(bg.resize((s, s)), (k * s, 0))
    sh.paste(Image.fromarray((m * 255).astype(np.uint8)).convert("RGB").resize((s, s)), (3 * s, 0))
    sh.save(out)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--name")
    ap.add_argument("--a")
    ap.add_argument("--b")
    ap.add_argument("-o", "--out")
    args = ap.parse_args()
    if args.name:
        picks = json.loads((ROOT / "art" / args.name / "picks.json").read_text())
        L = ROOT / "characters" / args.name / "layers"
        work = ROOT / "art" / args.name / "work" / "pair"
        work.mkdir(parents=True, exist_ok=True)
        for fb, spec in picks.get("_pair", {}).items():
            fa = spec if isinstance(spec, str) else spec["from"]
            a, b = load(L / f"{fa}.png"), load(L / f"{fb}.png")
            o, m = pair(a, b)
            for x0, y0, x1, y1 in ([] if isinstance(spec, str) else spec.get("keep_a", [])):
                o[y0:y1, x0:x1] = a[y0:y1, x0:x1]
                m[y0:y1, x0:x1] = 0
            save(L / f"{fb}.png", o)
            compare_sheet(a, b, o, m, work / f"{Path(fb).name}_cmp.png")
            print(f"짝 맞춤: {fb} <- {fa} (움직이는 부위 {float((m > 0.5).mean()) * 100:.1f}%)")
        return
    a, b = load(Path(args.a)), load(Path(args.b))
    o, m = pair(a, b)
    out = Path(args.out)
    save(out, o)
    compare_sheet(a, b, o, m, out.with_name(out.stem + "_cmp.png"))


if __name__ == "__main__":
    main()
