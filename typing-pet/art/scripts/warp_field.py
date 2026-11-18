"""캐릭터 움직임용 변형장 mask/warp 를 만든다 (docs/MOTION_QA.md 2절).

앱은 몸/머리 움직임을 레이어를 잘라 옮기지 않고, 합성된 그림 전체를 이 가중치만큼 밀어서 그린다.
가중치가 연속이라 잘린 경계(이음매, 반투명 선)가 생기지 않는다.

- R (몸): 책상(base_clean 의 불투명한 곳)에서 멀수록 1. 책상 위(키보드 위 앞발 포함)는 0 이라 제자리.
- G (머리): 몸 가중치 x 세로 경사 (머리 기준점 위는 1, 책상 윗면으로 내려갈수록 0).
- B: 안 쓴다 (0). 마우스는 변형이 아니라 팔/마우스 그림을 따로 움직인다 (mouse_rig.py).

characters/<이름>/layers/mask/warp.png 를 쓰고 manifest.toml 의 layers, [masks] 에 warp 를 넣는다 (이미 있으면 그대로).
사용법: warp_field.py --name usagi   (art/scripts/build.sh 가 slice.py 다음에 부른다)
"""

import argparse
import re
import tomllib
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA")).astype(np.float32)


def smoothstep(e0: float, e1: float, x: np.ndarray) -> np.ndarray:
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def dist_to(mask: np.ndarray) -> np.ndarray:
    """mask 픽셀까지의 거리 (mask 안은 0)."""
    return cv2.distanceTransform((~mask).astype(np.uint8), cv2.DIST_L2, 5)


def over_mask(dst: np.ndarray, layer: np.ndarray, m: np.ndarray) -> np.ndarray:
    """앱의 "마스크 안을 교체" 합성 (straight alpha 근사)."""
    m = m[..., None]
    return dst * (1 - m) + layer * m


def fill_holes(m: np.ndarray) -> np.ndarray:
    u = m.astype(np.uint8) * 255
    h, w = u.shape
    ff = u.copy()
    cv2.floodFill(ff, np.zeros((h + 2, w + 2), np.uint8), (0, 0), 255)
    return (u | cv2.bitwise_not(ff)) > 0


def body_head(base_clean: np.ndarray, head_y: float, s: float) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """(몸 가중치, 머리 가중치, 책상 마스크). s 는 1024 캔버스 대비 배율."""
    desk = base_clean[..., 3] > 128
    d = dist_to(desk)
    wb = smoothstep(4 * s, 96 * s, d)
    h, w = desk.shape
    cx = w // 2
    cols = desk[:, cx - int(150 * s): cx + int(150 * s)]
    ys = np.nonzero(cols.any(axis=1))[0]
    desk_top = float(ys.min()) if len(ys) else h * 0.7
    yy = np.mgrid[:h, :w][0].astype(np.float32)
    ramp = 1.0 - smoothstep(head_y, max(desk_top, head_y + 40 * s), yy)
    return wb, wb * ramp, desk


def patch_manifest(path: Path) -> None:
    t = path.read_text()
    # 예전 [swing] (변형장으로 마우스를 밀던 때의 값) 은 지운다. 마우스는 mouse_rig.py 의 [mouse] 가 맡는다
    t = re.sub(r"\n# 마우스 연출[^\n]*\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    t = re.sub(r"\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    if '"mask/warp"' not in t:
        t = re.sub(r"layers = \[", 'layers = [\n  "mask/warp",', t, count=1)
    if not re.search(r"^warp = ", t, re.M):
        t = re.sub(r"^\[masks\]\n", '[masks]\nwarp = "mask/warp"\n', t, count=1, flags=re.M)
    path.write_text(t)


def build(name: str) -> None:
    cdir = ROOT / "characters" / name
    m = tomllib.loads((cdir / "manifest.toml").read_text())
    L = cdir / "layers"
    clean = load(L / "base_clean.png")
    s = clean.shape[0] / 1024
    wb, wh, _ = body_head(clean, float(m["head_anchor"][1]), s)
    out = np.zeros(clean.shape, np.uint8)
    out[..., 0] = np.round(wb * 255)
    out[..., 1] = np.round(wh * 255)
    out[..., 3] = 255
    Image.fromarray(out).save(L / "mask/warp.png")
    patch_manifest(cdir / "manifest.toml")
    print(f"{name}: mask/warp 저장")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", required=True)
    build(ap.parse_args().name)


if __name__ == "__main__":
    main()
