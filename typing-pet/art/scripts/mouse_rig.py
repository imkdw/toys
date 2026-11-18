"""마우스 자세 리그를 만든다 (docs/MOTION_QA.md 2절 "마우스").

마우스를 쓰는 동안 앱은 세 장을 겹쳐 그린다. 그림을 문질러 밀지 않으므로 손과 마우스가 찌그러지지 않는다.
- paw_r/mouse: 오른앞발을 지운 몸과 마우스를 치운 빈 패드 (앞발 마스크 안을 교체).
- fx/mouse: 마우스만. 손과 같이 패드 위를 움직인다.
- fx/mouse_arm: 팔과 손. 어깨(root)는 몸에 붙어 있고 손(tip)이 마우스를 따라가도록 앱이 root -> tip 축을 따라 기울여 그린다.

팔은 base 의 선 색/굵기와 몸 색을 재서 그린다 (생성 그림마다 팔 길이와 모양이 달라 캐릭터끼리 맞지 않았다).
manifest.toml 에 [mouse] (root, tip, hand, range) 를 쓴다. art/<이름>/picks.json 의 "_mouse" 로 값을 덮어쓴다:
  root, tip_offset, bow, radius, fill, line, line_width, erase (지울 앞발 상자), keep (지우지 않을 상자)

사용법: mouse_rig.py --name usagi [--preview out.png]   (art/scripts/build.sh 가 warp_field.py 다음에 부른다)
"""

import argparse
import json
import re
import tomllib
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SS = 4  # 팔을 그릴 때의 슈퍼샘플 배율
# 손이 마우스와 한 덩어리로 움직이기 시작하는 곳 (root 0 ~ tip 1)
HAND = 0.72


def load(p: Path) -> np.ndarray:
    return np.array(Image.open(p).convert("RGBA")).astype(np.float32)


def save(p: Path, a: np.ndarray) -> None:
    p.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(np.clip(np.round(a), 0, 255).astype(np.uint8)).save(p)


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def dil(m: np.ndarray, k: int) -> np.ndarray:
    return cv2.dilate(m.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (k | 1, k | 1))) > 0


def fill_holes(m: np.ndarray) -> np.ndarray:
    u = m.astype(np.uint8) * 255
    h, w = u.shape
    ff = u.copy()
    cv2.floodFill(ff, np.zeros((h + 2, w + 2), np.uint8), (0, 0), 255)
    return (u | cv2.bitwise_not(ff)) > 0


def is_dark(rgba: np.ndarray, thr: float = 140) -> np.ndarray:
    return (rgba[..., :3].max(axis=-1) < thr) & (rgba[..., 3] > 128)


def find_mouse(clean: np.ndarray, hint: list[float]) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """(마우스 + 외곽선, 패드 + 테두리, 마우스 중심). base_clean 에서 hint 근처의 흰 덩어리."""
    rgb = clean[..., :3]
    opaque = clean[..., 3] > 128
    white = (rgb.min(axis=-1) > 215) & opaque
    n, lab, st, c = cv2.connectedComponentsWithStats(white.astype(np.uint8), 8)
    cands = [j for j in range(1, n) if 800 < st[j, cv2.CC_STAT_AREA] < 9000 and np.hypot(*(c[j] - hint)) < 90]
    if not cands:
        raise SystemExit(f"마우스를 못 찾음 (hint={hint})")
    j = min(cands, key=lambda j: np.hypot(*(c[j] - hint)))
    body = fill_holes(lab == j)
    dark = is_dark(clean)
    # 외곽선: 마우스에 맞닿은 진한 선 중 마우스 둘레 안쪽만 (패드 테두리와 이어져 있어도 마우스 둘레만 남는다)
    ring = dil(body, 21) & dark
    full = fill_holes(body | ring)
    full = (cv2.morphologyEx(full.astype(np.uint8), cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (15, 15))) > 0) | body
    full = dil(full, 3) & (dark | full)
    full = fill_holes(full)
    around = dil(full, 13) & ~dil(full, 5) & ~dark
    pad_col = np.median(rgb[around], axis=0)
    padlike = (np.abs(rgb - pad_col).max(axis=-1) < 26) & opaque
    n, lab, _, _ = cv2.connectedComponentsWithStats((padlike | full).astype(np.uint8), 8)
    cy, cx = int(c[j][1]), int(c[j][0])
    pad = fill_holes(lab == lab[cy, cx])
    return full, pad, np.array(c[j])


def empty_pad(clean: np.ndarray, mouse: np.ndarray, pad: np.ndarray) -> np.ndarray:
    """마우스를 지운 책상. 마우스가 걸쳐 있던 패드 윗변은 양옆 선을 이어 다시 긋는다."""
    out = clean.copy()
    dark = is_dark(clean)
    hole = dil(mouse, 7)
    ys, xs = np.nonzero(hole)
    x0, x1 = xs.min(), xs.max()
    # 패드 윗변: 구멍 양옆에서 패드 안쪽(색이 있는 곳)이 시작하는 높이를 직선으로 잇는다. 둥근 모서리는 뺀다
    rim = dark & dil(pad, 9) & ~hole
    inner = pad & ~dark
    pxs = np.nonzero(inner.any(axis=0))[0]
    pts = []
    for x in range(pxs.min() + 14, pxs.max() - 14):
        if x0 - 2 <= x <= x1 + 2:
            continue
        col = np.nonzero(inner[:, x])[0]
        top = col.min()
        pts.append((x, top, int(dark[top - 14:top, x].sum())))
    pts = np.array(pts, np.float32)
    keep = np.abs(pts[:, 1] - np.median(pts[:, 1])) < 2.5
    k, b = np.polyfit(pts[keep, 0], pts[keep, 1], 1)
    lw = float(np.median(pts[keep, 2]))
    b -= lw / 2 * np.sqrt(1 + k * k)  # 선 가운데
    rgb = clean[..., :3]
    near = dil(hole, 25) & ~hole & ~dark & (clean[..., 3] > 128)
    yy, xx = np.mgrid[:clean.shape[0], :clean.shape[1]]
    sd = (yy - (k * xx + b)) / np.sqrt(1 + k * k)  # 윗변에서 아래로 +
    pad_col = np.median(rgb[near & pad & (sd > lw)], axis=0)
    desk_col = np.median(rgb[near & ~dil(pad, 9) & (sd < -lw)], axis=0)
    line_col = np.median(rgb[rim], axis=0)
    a_line = 1.0 - smoothstep(lw / 2 - 0.7, lw / 2 + 0.7, np.abs(sd))
    a_pad = smoothstep(-0.7, 0.7, sd)
    fill = desk_col[None, None] * (1 - a_pad[..., None]) + pad_col[None, None] * a_pad[..., None]
    fill = fill * (1 - a_line[..., None]) + line_col[None, None] * a_line[..., None]
    w = cv2.GaussianBlur(dil(mouse, 9).astype(np.float32), (0, 0), 1.2)[..., None]
    out[..., :3] = rgb * (1 - w) + fill * w
    return out


def erase_paw(base: np.ndarray, clean: np.ndarray, pmask: np.ndarray, paw_tip: list[float], cfg: dict) -> np.ndarray:
    """오른앞발을 지운 그림: 몸 위의 앞발 선은 몸 색으로 메우고, 책상에 걸친 부분은 빈 책상으로."""
    h, w = pmask.shape
    yy, xx = np.mgrid[:h, :w]
    desk = clean[..., 3] > 128
    body = (base[..., 3] > 128) & ~desk
    # 몸 안쪽의 선: 바깥(투명)에서 떨어진 진한 픽셀. 윤곽선은 남긴다
    outside = base[..., 3] < 128
    d_out = cv2.distanceTransform((~outside).astype(np.uint8), cv2.DIST_L2, 5)
    dark = is_dark(base, 170)
    # 앞발 = 앞발 자리에 걸친 몸 안쪽 선 덩어리 (팔 선이 이어져 있으면 같이 지운다)
    cand = dark & body & (d_out > 12) & (pmask > 0.5)
    for kb in cfg.get("keep", []):
        cand &= ~((xx >= kb[0]) & (xx <= kb[2]) & (yy >= kb[1]) & (yy <= kb[3]))
    n, lab, _, _ = cv2.connectedComponentsWithStats(cand.astype(np.uint8), 8)
    seed = (xx - paw_tip[0]) ** 2 + (yy - (paw_tip[1] - 30)) ** 2 < 45 ** 2
    inner = np.isin(lab, [j for j in np.unique(lab[seed]) if j > 0])
    for box in cfg.get("erase", []):
        inner |= cand & (xx >= box[0]) & (xx <= box[2]) & (yy >= box[1]) & (yy <= box[3])
    zone = dil(inner, 41) & (pmask > 0.5)
    # 책상 선과 맞닿은 아래쪽까지: 책상 위(base_clean 과 달라진 곳)
    diff = np.abs(base[..., :3] - clean[..., :3]).max(axis=-1)
    on_desk = desk & (diff > 30) & zone
    on_desk = cv2.morphologyEx(on_desk.astype(np.uint8), cv2.MORPH_OPEN, np.ones((3, 3), np.uint8)) > 0
    # 앞발에 이어진 덩어리만 (키 빛처럼 떨어져 있는 차이는 그대로)
    n, lab, _, _ = cv2.connectedComponentsWithStats((on_desk | dil(inner, 9)).astype(np.uint8), 8)
    on_desk &= np.isin(lab, [j for j in np.unique(lab[inner]) if j > 0])
    out = base.copy()
    hole = dil(inner, 11) & body
    rgb = np.ascontiguousarray(np.clip(out[..., :3], 0, 255).astype(np.uint8))
    # 몸 색으로 메우기: 구멍 둘레에 책상 선이 있으면 그 색이 번지므로, 책상 쪽은 먼저 몸 색으로 덮은 사본에서 메운다
    src = rgb.copy()
    body_col = np.median(rgb[dil(hole, 31) & body & ~dark & ~hole], axis=0)
    src[~body | (dark & ~hole)] = body_col
    filled = cv2.inpaint(src, hole.astype(np.uint8), 7, cv2.INPAINT_TELEA).astype(np.float32)
    wgt = cv2.GaussianBlur(hole.astype(np.float32), (0, 0), 1.0)[..., None] * body[..., None]
    out[..., :3] = out[..., :3] * (1 - wgt) + filled * wgt
    # 책상 위: 빈 책상으로 (앞발이 덮었던 책상 뒤 선, 키보드 윗줄이 돌아온다)
    wd = cv2.GaussianBlur(dil(on_desk, 31).astype(np.float32), (0, 0), 2.0)
    wd = np.where(desk, wd, 0.0)[..., None]
    out = out * (1 - wd) + clean * wd
    return out, hole | (wd[..., 0] > 0.5)


def bezier(p0, p1, p2, n=160):
    t = np.linspace(0, 1, n)[:, None]
    return (1 - t) ** 2 * p0 + 2 * (1 - t) * t * p1 + t ** 2 * p2


def draw_arm(shape, root, tip, bow, r_arm, r_paw, lw, fill, line, tip_col=None):
    """팔과 손 (RGBA, straight alpha). 뿌리는 몸 색에 녹아들고, 양옆 선은 둥근 끝으로 시작한다."""
    root, tip = np.array(root, np.float32), np.array(tip, np.float32)
    ax = tip - root
    L = float(np.hypot(*ax))
    nrm = np.array([ax[1], -ax[0]]) / L  # 오른쪽 위(바깥) 방향
    ctrl = (root + tip) / 2 + nrm * bow
    pts = bezier(root, ctrl, tip)
    seg = np.hypot(*np.diff(pts, axis=0).T)
    s = np.concatenate([[0], np.cumsum(seg)]) / seg.sum()
    rad = r_arm + (r_paw - r_arm) * smoothstep(0.45, 0.95, s)
    pad = int(r_paw + lw + 6)
    x0, y0 = int(pts[:, 0].min()) - pad, int(pts[:, 1].min()) - pad
    x1, y1 = int(pts[:, 0].max()) + pad, int(pts[:, 1].max()) + pad
    gy, gx = np.mgrid[y0 * SS:y1 * SS, x0 * SS:x1 * SS]
    px = (gx.astype(np.float32) + 0.5) / SS
    py = (gy.astype(np.float32) + 0.5) / SS
    sdf = np.full(px.shape, 1e9, np.float32)
    sbest = np.zeros(px.shape, np.float32)
    for i in range(len(pts)):
        dd = np.hypot(px - pts[i, 0], py - pts[i, 1]) - rad[i]
        m = dd < sdf
        sdf[m] = dd[m]
        sbest[m] = s[i]
    aa = 0.5 / SS * 2
    inside = 1.0 - smoothstep(-aa, aa, sdf)
    # 선: 가장자리 안쪽 lw
    edge = smoothstep(-lw - aa, -lw + aa, sdf)
    s_line = 0.20
    line_on = smoothstep(s_line - 0.01, s_line + 0.01, sbest)
    # 선의 둥근 시작점 (양옆)
    i0 = int(np.searchsorted(s, s_line))
    tan = pts[min(i0 + 1, len(pts) - 1)] - pts[i0 - 1]
    tan /= np.hypot(*tan)
    nn = np.array([tan[1], -tan[0]])
    for sgn in (1, -1):
        c = pts[i0] + nn * sgn * (rad[i0] - lw / 2)
        cap = 1.0 - smoothstep(lw / 2 - aa, lw / 2 + aa, np.hypot(px - c[0], py - c[1]))
        line_on = np.maximum(line_on, cap * (sbest <= s_line + 0.02))
    a_line = edge * line_on
    # 속: 뿌리에서 서서히 (몸 색과 같아서 경계가 안 보인다)
    # 선이 시작하기 전에는 속을 그리지 않는다 (몸 색과 같아서 필요 없고, 그리면 몸 윤곽선을 지운다). 바로 앞에서만 서서히
    a_fill = smoothstep(s_line - 0.07, s_line, sbest)
    fill_rgb = np.array(fill, np.float32)[None, None]
    if tip_col is not None:
        # 손(진한 색)과 팔의 경계는 팔 가운데가 뿌리 쪽으로 볼록한 둥근 선
        lat = np.clip((sdf + np.interp(sbest, s, rad)) / np.interp(sbest, s, rad), 0, 1)
        edge_s = 0.42 + 0.16 * lat * lat
        k = smoothstep(edge_s, edge_s + 1.5 / L, sbest)[..., None]
        fill_rgb = fill_rgb * (1 - k) + np.array(tip_col, np.float32)[None, None] * k
    rgb = fill_rgb * (1 - a_line[..., None]) + np.array(line, np.float32)[None, None] * a_line[..., None]
    alpha = inside * np.maximum(a_fill, a_line)
    # 선 시작 전에는 가장자리를 부드럽게 (속만 있는 곳)
    soft_w = 8.0 * (1.0 - smoothstep(s_line - 0.07, s_line, sbest)) + 0.01
    soft = 1.0 - smoothstep(-soft_w, 0.0, sdf)
    alpha = np.where(sbest < s_line, np.maximum(inside * a_line, a_fill * soft), alpha)
    small_rgb = cv2.resize(rgb * alpha[..., None], (x1 - x0, y1 - y0), interpolation=cv2.INTER_AREA)
    small_a = cv2.resize(alpha, (x1 - x0, y1 - y0), interpolation=cv2.INTER_AREA)
    out = np.zeros((*shape, 4), np.float32)
    out[y0:y1, x0:x1, :3] = small_rgb / np.maximum(small_a[..., None], 1e-4)
    out[y0:y1, x0:x1, 3] = small_a * 255
    return out


def over(dst: np.ndarray, src: np.ndarray) -> np.ndarray:
    a = src[..., 3:4] / 255.0
    out = dst.copy()
    out[..., :3] = dst[..., :3] * (1 - a) + src[..., :3] * a
    out[..., 3:4] = dst[..., 3:4] * (1 - a) + 255.0 * a
    return out


def bend(img: np.ndarray, root, tip, d, hand=HAND) -> np.ndarray:
    """앱과 같은 팔 기울이기: root 는 제자리, hand 이후는 d 만큼, 사이는 부드럽게."""
    h, w = img.shape[:2]
    yy, xx = np.mgrid[:h, :w].astype(np.float32)
    ax = np.array(tip, np.float32) - np.array(root, np.float32)
    qx, qy = xx.copy(), yy.copy()
    for _ in range(6):
        s = ((qx - root[0]) * ax[0] + (qy - root[1]) * ax[1]) / (ax @ ax) / hand
        t = arm_t(s)
        qx, qy = xx - t * d[0], yy - t * d[1]
    pm = img.copy()
    pm[..., :3] *= pm[..., 3:4] / 255.0
    o = cv2.remap(pm, qx, qy, cv2.INTER_LINEAR, borderValue=0)
    o[..., :3] /= np.maximum(o[..., 3:4] / 255.0, 1e-4)
    return o


def arm_t(s):
    """축 위 위치 s (0 root ~ 1 손 시작) -> 이동 비율. 양 끝이 부드러운 직선."""
    s = np.clip(s, 0.0, 1.0)
    return 0.5 * s + 0.5 * (s * s * (3 - 2 * s))


def shift(img: np.ndarray, d) -> np.ndarray:
    m = np.float32([[1, 0, d[0]], [0, 1, d[1]]])
    pm = img.copy()
    pm[..., :3] *= pm[..., 3:4] / 255.0
    o = cv2.warpAffine(pm, m, (img.shape[1], img.shape[0]), flags=cv2.INTER_LINEAR)
    o[..., :3] /= np.maximum(o[..., 3:4] / 255.0, 1e-4)
    return o


def travel(mouse: np.ndarray, pad: np.ndarray) -> list[list[float]]:
    """마우스가 패드 안에서 갈 수 있는 범위 [[왼, 오른], [위, 아래]] (px). 아래쪽 2/3 가 패드 안에 남는 만큼."""
    ys, xs = np.nonzero(mouse)
    low = mouse & (np.mgrid[:mouse.shape[0], :mouse.shape[1]][0] > ys.min() + (ys.max() - ys.min()) / 3)
    inner = cv2.erode(pad.astype(np.uint8), np.ones((5, 5), np.uint8)) > 0
    n = low.sum()

    def room(dx, dy, cap):
        best = 0
        for k in range(1, cap + 1):
            sh = np.roll(low, (dy * k, dx * k), axis=(0, 1))
            if (sh & inner).sum() < 0.985 * n:
                break
            best = k
        return best

    return [[-float(room(-1, 0, 30)), float(room(1, 0, 30))], [-float(max(room(0, -1, 12), 5)), float(room(0, 1, 12))]]


def patch_manifest(path: Path, info: dict) -> None:
    t = path.read_text()
    t = re.sub(r"\n# 마우스 연출[^\n]*\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    t = re.sub(r"\n\[swing\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    t = re.sub(r"\n# 마우스 리그[^\n]*\n\[mouse\]\n(?:[a-z_]+ = [^\n]*\n)*", "\n", t)
    for old in ('  "paw_r/mouse_bg",\n', '  "paw_r/mouse_click",\n'):
        t = t.replace(old, "")
    for name in ("fx/mouse", "fx/mouse_arm", "paw_r/mouse"):
        if f'"{name}"' not in t:
            t = re.sub(r"layers = \[", f'layers = [\n  "{name}",', t, count=1)
    t = re.sub(r'^"paw_r/mouse(_click)?" = [^\n]*\n', "", t, flags=re.M)
    r = info["range"]
    block = (
        "\n# 마우스 리그: fx/mouse_arm 의 어깨(root)는 제자리, 손(tip)과 fx/mouse 가 range 안에서 같이 움직인다 (캔버스 px, mouse_rig.py)\n"
        f"[mouse]\nroot = [{info['root'][0]:.1f}, {info['root'][1]:.1f}]\ntip = [{info['tip'][0]:.1f}, {info['tip'][1]:.1f}]\n"
        f"hand = {HAND}\nrange = [[{r[0][0]}, {r[0][1]}], [{r[1][0]}, {r[1][1]}]]\n"
    )
    # [tips] 뒤의 첫 섹션 앞에 넣는다 (배열 테이블 [[scene]] 뒤에 오면 그 안으로 들어간다)
    m = re.search(r"^\[(timing|anim|expressions|desk)\]|^\[\[", t, re.M)
    cut = m.start() if m else len(t)
    head = t[:cut].rstrip("\n") + "\n"
    # 설명 주석이 바로 앞에 붙은 섹션이면 주석 앞에
    lines = head.split("\n")
    tail_comments = []
    while len(lines) > 1 and lines[-2].startswith("#"):
        tail_comments.insert(0, lines.pop(-2))
    head = "\n".join(lines).rstrip("\n") + "\n"
    t = head + block + "\n" + "".join(c + "\n" for c in tail_comments) + t[cut:]
    path.write_text(t)


def build(name: str, preview: str | None = None, write: bool = True) -> dict:
    cdir = ROOT / "characters" / name
    man = tomllib.loads((cdir / "manifest.toml").read_text())
    picks_p = ROOT / "art" / name / "picks.json"
    cfg = json.loads(picks_p.read_text()).get("_mouse", {}) if picks_p.exists() else {}
    L = cdir / "layers"
    base, clean = load(L / "base.png"), load(L / "base_clean.png")
    pmask = load(L / "mask/paw_r.png")[..., 0] / 255.0
    paw_tip = man["tips"]["paw_r/down"]
    mouse, pad, mc = find_mouse(clean, cfg.get("mouse_hint", [757, 752]))
    desk0 = empty_pad(clean, mouse, pad)
    nopaw, erased = erase_paw(base, clean, pmask, paw_tip, cfg)
    # 바탕: 앞발 지운 그림 + 빈 패드 (책상 위는 base 와 같은 곳만 빈 패드로)
    wpad = cv2.GaussianBlur(dil(mouse, 17).astype(np.float32), (0, 0), 1.5)[..., None]
    bg = nopaw * (1 - wpad) + desk0 * wpad
    # 마우스 스프라이트
    ma = cv2.GaussianBlur(mouse.astype(np.float32), (0, 0), 0.6)
    spr = clean.copy()
    spr[..., 3] = ma * 255
    # 팔: 색과 굵기를 base 에서 잰다
    dark = is_dark(base)
    body = (base[..., 3] > 128) & (clean[..., 3] < 128)
    yy, xx = np.mgrid[:base.shape[0], :base.shape[1]]
    line = cfg.get("line") or np.median(base[..., :3][dark & body & (np.hypot(xx - paw_tip[0], yy - paw_tip[1] + 30) < 90)], axis=0).tolist()
    tip = (mc + np.array(cfg.get("tip_offset", [-10.0, -37.0]))).tolist()
    root = cfg.get("root")
    if root is None:
        # 책상 뒤 선 위쪽, 몸 오른쪽 가장자리에서 안으로 들어온 곳
        y_r = tip[1] - cfg.get("drop", 84)
        row = np.nonzero(body[int(y_r)])[0]
        root = [float(min(row.max() - cfg.get("inset", 40), tip[0] - 58)), float(y_r)]
    near_root = (np.hypot(xx - root[0], yy - root[1]) < 26) & body & ~is_dark(base, 190)
    fill = cfg.get("fill") or np.median(base[..., :3][near_root], axis=0).tolist()
    r_paw = cfg.get("radius", 27.0)
    arm = draw_arm(base.shape[:2], root, tip, cfg.get("bow", 9.0), cfg.get("radius_arm", r_paw - 2.0), r_paw, cfg.get("line_width", 7.5), fill, line, line if cfg.get("tip_fill") == "line" else cfg.get("tip_fill"))
    rng = cfg.get("range") or travel(mouse, pad)
    info = {"root": root, "tip": tip, "range": rng, "center": mc.tolist()}
    if write:
        out_bg = bg.copy()
        out_bg[..., 3] = np.where(pmask > 0, out_bg[..., 3], 0)
        save(L / "paw_r/mouse.png", out_bg)
        save(L / "fx/mouse.png", spr)
        save(L / "fx/mouse_arm.png", arm)
        for old in ("paw_r/mouse_bg.png", "paw_r/mouse_click.png"):
            (L / old).unlink(missing_ok=True)
        patch_manifest(cdir / "manifest.toml", info)
    if preview:
        full_bg = base * (1 - pmask[..., None]) + bg * pmask[..., None]
        tiles = [base]
        for d in ([0, 0], [rng[0][0], 0], [rng[0][1], 0], [0, rng[1][0]], [0, rng[1][1]], [rng[0][0], rng[1][1]], [rng[0][1], rng[1][0]]):
            f = over(full_bg, shift(spr, d))
            tiles.append(over(f, bend(arm, root, tip, d)))
        tiles.insert(1, full_bg)
        box = (500, 540, 850, 840)
        sheet = np.zeros(((box[3] - box[1]) * 2, (box[2] - box[0]) * 2 * len(tiles), 3), np.uint8)
        for i, t in enumerate(tiles):
            bgc = np.zeros_like(t)
            bgc[..., :3] = (40, 44, 52)
            bgc[..., 3] = 255
            c = over(bgc, t)[box[1]:box[3], box[0]:box[2], :3]
            c = cv2.resize(np.clip(c, 0, 255).astype(np.uint8), None, fx=2, fy=2, interpolation=cv2.INTER_LANCZOS4)
            sheet[:, i * c.shape[1]:(i + 1) * c.shape[1]] = c
        Image.fromarray(sheet).save(preview)
    print(f"{name}: 마우스 리그 root={[round(v, 1) for v in root]} tip={[round(v, 1) for v in tip]} range={rng} 선 {[round(v) for v in line]} 속 {[round(v) for v in fill]}")
    return info


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", required=True)
    ap.add_argument("--preview")
    ap.add_argument("--dry", action="store_true", help="파일을 쓰지 않고 미리보기만")
    a = ap.parse_args()
    build(a.name, a.preview, not a.dry)


if __name__ == "__main__":
    main()
