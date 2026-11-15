"""캐릭터 움직임용 변형장 mask/warp 를 만든다 (docs/MOTION_QA.md 2절).

앱은 몸/머리/마우스 움직임을 레이어를 잘라 옮기지 않고, 합성된 그림 전체를 이 가중치만큼 밀어서 그린다.
가중치가 연속이라 잘린 경계(이음매, 반투명 선)가 생기지 않는다.

- R (몸): 책상(base_clean 의 불투명한 곳)에서 멀수록 1. 책상 위(키보드 위 앞발, 마우스 팔 포함)는 0 이라 제자리.
- G (머리): 몸 가중치 x 세로 경사 (머리 기준점 위는 1, 책상 윗면으로 내려갈수록 0).
- B (마우스): 손과 마우스는 1, 팔은 어깨 쪽으로 갈수록 0, 패드 테두리/키보드/몸 윤곽 등 다른 선은 0.

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


def mouse_weight(base: np.ndarray, paw: np.ndarray, pmask: np.ndarray, tip: list[float], s: float) -> tuple[np.ndarray, dict]:
    """마우스 가중치와 정보 (마우스 덩어리, 패드 여백 px)."""
    h, w = pmask.shape
    yy, xx = np.mgrid[:h, :w]
    comp = over_mask(base, paw, pmask)
    rgb_b, rgb_m = base[..., :3], comp[..., :3]
    # 1) 마우스: base 에서 손끝 근처의 흰 덩어리
    white = (rgb_b.min(axis=-1) > 215) & (base[..., 3] > 128)
    n, lab, st, c = cv2.connectedComponentsWithStats(white.astype(np.uint8), 8)
    cands = [j for j in range(1, n) if 200 * s * s < st[j, cv2.CC_STAT_AREA] < 9000 * s * s and np.hypot(*(c[j] - tip)) < 90 * s]
    if not cands:
        raise SystemExit(f"마우스를 못 찾음 (tip={tip})")
    mouse = fill_holes(lab == min(cands, key=lambda j: np.hypot(*(c[j] - tip))))
    mc = np.array(c[min(cands, key=lambda j: np.hypot(*(c[j] - tip)))])
    # 외곽선까지 (진한 선 두께 만큼)
    dark_b = (rgb_b.max(axis=-1) < 130) & (base[..., 3] > 128)
    ring = cv2.dilate(mouse.astype(np.uint8), np.ones((int(15 * s) | 1,) * 2, np.uint8)) > 0
    mouse_full = fill_holes(mouse | (ring & dark_b))
    # 패드 테두리처럼 가늘게 붙은 선은 떼어낸다 (마우스는 두꺼운 덩어리라 남는다)
    k = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (int(13 * s) | 1,) * 2)
    mouse_full = (cv2.morphologyEx(mouse_full.astype(np.uint8), cv2.MORPH_OPEN, k) > 0) | mouse
    # 2) 패드: 마우스 둘레의 패드 색과 비슷한 이어진 영역 + 마우스
    around = (cv2.dilate(mouse_full.astype(np.uint8), np.ones((int(9 * s) | 1,) * 2, np.uint8)) > 0) & ~mouse_full
    pad_col = np.median(rgb_b[around & ~dark_b], axis=0)
    padlike = (np.abs(rgb_b - pad_col).max(axis=-1) < 24) & (base[..., 3] > 128)
    n, lab, st, _ = cv2.connectedComponentsWithStats((padlike | mouse_full).astype(np.uint8), 8)
    pad = fill_holes(lab == lab[int(mc[1]), int(mc[0])])
    margin = float(dist_to(~pad)[mouse_full].min()) if pad.any() else 0.0
    # 3) 팔: 마우스 자세가 base 와 달라진 곳 중 손끝에 이어진 덩어리
    diff = np.abs(rgb_m - rgb_b).max(axis=-1) + np.abs(comp[..., 3] - base[..., 3])
    chg = (diff > 40) & (pmask > 0.3)
    chg = cv2.morphologyEx(chg.astype(np.uint8), cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
    n, lab, st, c = cv2.connectedComponentsWithStats(chg, 8)
    near = (xx - mc[0]) ** 2 + (yy - mc[1]) ** 2 < (70 * s) ** 2
    ids = [j for j in range(1, n) if st[j, cv2.CC_STAT_AREA] > 150 * s * s and (lab[near] == j).any()]
    arm = np.isin(lab, ids)
    arm = cv2.morphologyEx(arm.astype(np.uint8), cv2.MORPH_CLOSE, np.ones((int(9 * s) | 1,) * 2, np.uint8)) > 0
    # 4) 1 인 곳: 마우스 + 마우스에 붙은 손. 0 인 곳: 손/마우스/팔에 속하지 않은 진한 선, 패드 테두리, 팔/패드에서 먼 곳.
    #    손 근처의 가로 선(패드 윗변, 책상 뒤 선)은 비워 둔다: 손은 주로 옆으로 미끄러지므로 가로 선을 따라 밀어도 안 보이고,
    #    0 으로 묶으면 손 바로 옆에서 가중치가 급하게 바뀌어 접힌다.
    hand = arm & (cv2.dilate(mouse_full.astype(np.uint8), np.ones((int(41 * s) | 1,) * 2, np.uint8)) > 0)
    one = mouse_full | hand
    dark_m = (rgb_m.max(axis=-1) < 130) & (comp[..., 3] > 100)
    sprite = cv2.dilate((arm | mouse_full).astype(np.uint8), np.ones((5, 5), np.uint8)) > 0
    zone = pad | (cv2.dilate(arm.astype(np.uint8), np.ones((int(41 * s) | 1,) * 2, np.uint8)) > 0)
    lines = dark_m & ~sprite & ~one
    lb = cv2.GaussianBlur(lines.astype(np.float32), (0, 0), 2.0 * s)
    lgx, lgy = cv2.Sobel(lb, cv2.CV_32F, 1, 0, ksize=5), cv2.Sobel(lb, cv2.CV_32F, 0, 1, ksize=5)
    lgx, lgy = cv2.GaussianBlur(np.abs(lgx), (0, 0), 3 * s), cv2.GaussianBlur(np.abs(lgy), (0, 0), 3 * s)
    d_one = dist_to(one)
    free = ((lgy > 2.5 * lgx) & (d_one < 40 * s)) | (d_one < 20 * s)
    zero = (lines & ~free) | ~zone
    zero &= ~one
    dz, do = dist_to(zero), dist_to(one)
    wm = dz / np.maximum(dz + do, 1e-3)
    wm = cv2.GaussianBlur(wm.astype(np.float32), (0, 0), 2.0 * s)
    wm[one & (dz > 3 * s)] = np.maximum(wm[one & (dz > 3 * s)], 1.0)
    wm[zero] = 0.0
    wm = cv2.GaussianBlur(wm, (0, 0), 1.0 * s)
    # 마우스에서 40~90px 사이에서 사라지게: 팔 아래쪽(손목)만 늘어나고 위팔과 그 옆 선은 제자리
    wm *= 1.0 - smoothstep(40 * s, 90 * s, dist_to(mouse_full))
    wm = np.clip(wm, 0, 1)
    gy, gx = np.gradient(wm)
    # 접힘 없이 밀 수 있는 최대 변위 (기울기 x 변위 < 0.6)
    max_px = [round(0.4 / max(float(np.abs(gx).max()), 1e-3), 1), round(0.4 / max(float(np.abs(gy).max()), 1e-3), 1)]
    return wm, {"max_px": max_px, "mouse": mouse_full, "pad": pad, "margin": margin, "arm_px": int(arm.sum()), "center": mc.tolist()}


def pad_edge(pad: np.ndarray) -> np.ndarray:
    er = cv2.erode(pad.astype(np.uint8), np.ones((3, 3), np.uint8)) > 0
    return pad & ~er


def patch_manifest(path: Path, max_px: list | None) -> None:
    t = path.read_text()
    t = re.sub(r"\n# 마우스 연출[^\n]*\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    t = re.sub(r"\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    if max_px:
        t = t.rstrip("\n") + f"\n\n# 마우스 연출: mask/warp 의 B 채널만큼 손과 마우스를 민다. 접히지 않는 최대 변위 (캔버스 px, warp_field.py)\n[swing]\nmax_px = [{max_px[0]}, {max_px[1]}]\n"
    if '"mask/warp"' not in t:
        t = re.sub(r"layers = \[", 'layers = [\n  "mask/warp",', t, count=1)
    if not re.search(r"^warp = ", t, re.M):
        t = re.sub(r"^\[masks\]\n", '[masks]\nwarp = "mask/warp"\n', t, count=1, flags=re.M)
    path.write_text(t)


def build(name: str) -> dict:
    cdir = ROOT / "characters" / name
    m = tomllib.loads((cdir / "manifest.toml").read_text())
    L = cdir / "layers"
    base = load(L / "base.png")
    clean = load(L / "base_clean.png")
    canvas = base.shape[0]
    s = canvas / 1024
    wb, wh, desk = body_head(clean, float(m["head_anchor"][1]), s)
    tips = m.get("tips", {})
    tip = tips.get("paw_r/mouse_click") or tips.get("paw_r/mouse")
    wm = np.zeros_like(wb)
    info = {}
    if tip and (L / "paw_r/mouse.png").exists():
        pm = load(L / "mask/paw_r.png")[..., 0] / 255.0
        wm, info = mouse_weight(base, load(L / "paw_r/mouse.png"), pm, tip, s)
    out = np.zeros(base.shape, np.uint8)
    out[..., 0] = np.round(wb * 255)
    out[..., 1] = np.round(wh * 255)
    out[..., 2] = np.round(wm * 255)
    out[..., 3] = 255
    Image.fromarray(out).save(L / "mask/warp.png")
    patch_manifest(cdir / "manifest.toml", info.get("max_px"))
    margin = info.get("margin")
    print(f"{name}: mask/warp 저장" + (f", 마우스 패드 여백 {margin:.0f}px, 팔 {info['arm_px']}px, 최대 변위 {info['max_px']}" if info else ", 마우스 없음"))
    return info


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", required=True)
    build(ap.parse_args().name)


if __name__ == "__main__":
    main()
