"""움직임 품질 자동 판정 (docs/MOTION_QA.md 3절). scripts/motion_qa.sh 가 렌더한 뒤 부른다.

입력 (캐릭터마다 <out>/<이름>/):
  bhv/<behavior>/<번호>.png, <번호>_s.png (변형을 뺀 정지 프레임), frames.json   (--render-behaviors --pairs)
  tl_<시나리오>/<번호>.png, <번호>_s.png, timeline.json                        (--render-timeline --pairs)
출력: <out>/report.md, <out>/report.json, <out>/<이름>/sheet_*.png (눈 확인용)

사용법: motion_qa.py --out target/motion_qa --size 512 usagi chiikawa ...
"""

import argparse
import json
import sys
import tomllib
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).parent))
import warp_field as wf  # noqa: E402

GHOST_FRAC = 0.0005   # R2: 정지 프레임보다 늘어난 반투명 선 픽셀 비율 상한
COMP_PLUS = 2         # R4: 진한 선 덩어리 수 증가 상한
DESK_DIFF = 2.0       # R1: 책상 선 평균 차이 상한 (0~255)
SCENE_DESK_DIFF = 12.0  # R1b: 장면 프레임의 책상 선과 base 책상 선 평균 차이 상한
NCC_MIN = 0.9         # R3
MOUSE_MIN_PX = 16.0   # M1: mouse 시나리오에서 손+마우스 최대 변위 하한 (1024 캔버스 px)
DESK_MOVED = 0.001    # M4: 마우스 프레임에서 손/팔/마우스 밖 책상 픽셀 중 정지 프레임과 달라진 비율 상한
OVERSHOOT_MIN = 0.05  # M2: 커서가 멈춘 뒤 반대쪽으로 넘치는 크기 하한 (멈출 때 변위 대비)
FOLLOW_MIN = 0.6      # M3: 커서 이동 방향과 손 변위의 상관 하한 (손 늦음 0~2 프레임 중 가장 좋은 것)
FOLD_DET = (0.5, 2.0)  # F2: 샘플링 사상의 야코비안 행렬식 범위 (0 이하면 접힘, 0.5 미만/2 초과면 두 배 넘게 늘어나거나 눌림)
MIX_GHOST = 0.0005    # T1: 섞인 프레임이 한 장으로 고른 기준보다 늘어난 반투명 선 픽셀 비율 상한
PAIR_LINE_PX = 2.0    # T2: 움직이는 부위 밖 윤곽선 거리 95 백분위 상한 (캔버스 px)
LONE_PX = 15.0        # T2: 다른 그림에 이 거리 안에 짝 선이 없으면 옮겨 간 소품 선으로 보고 움직이는 부위에 넣는다
DISP_MAX = 12.0       # T3: 몸/머리 움직임의 캐릭터 픽셀 최대 변위 상한 (1024 캔버스 px)
GLOW_PAW = 3.0        # T4: 키 빛 근처 앞발 픽셀의 평균 색 차이 상한 (0~255)
ARM_RATIO = 1.5       # T5: 마우스 팔 길이 (어깨~손) / 쉬는 앞발 길이 상한 (몸 한가운데에서 뻗던 예전 팔은 2배 안팎)
DISP_MIN = 4.0        # T6: motion 이 있는 behavior 의 최대 변위 하한 (1024 캔버스 px, 512 렌더에서 2px)
TREMBLE_MIN_MS = 90   # T6: tremble 주기 하한 (이보다 짧으면 60fps 에서 화면 떨림으로 보임)
MOTION_DEFAULTS = {"hop": (6.0, 450), "tremble": (1.2, 70), "sway": (2.5, 1200), "nod": (3.0, 500), "squash": (0.0, 300)}  # src/config.rs MotionKind::defaults


def rgba(p: Path, size: int | None = None) -> np.ndarray:
    im = Image.open(p).convert("RGBA")
    if size and im.size[0] != size:
        im = im.resize((size, size), Image.LANCZOS)
    return np.array(im).astype(np.float32)


def flat(im: np.ndarray) -> np.ndarray:
    """흰 바탕에 합성한 RGB (투명 영역 차이를 색 차이로)."""
    a = im[..., 3:4] / 255.0
    return im[..., :3] * a + 255 * (1 - a)


def dark(im: np.ndarray) -> np.ndarray:
    return (im[..., 3] > 128) & (im[..., :3].max(axis=-1) < 110)


def ghost(im: np.ndarray) -> np.ndarray:
    """진한 중심 없이 반쯤 진한 무채색 선 (반투명으로 남은 윤곽선)."""
    rgb = flat(im)
    lum = rgb.mean(axis=-1)
    sat = rgb.max(axis=-1) - rgb.min(axis=-1)
    # 윤곽선(진한 갈색)이 흐려진 색: 무채색~갈색 (파란 머리 경계 같은 색 영역 가장자리는 빼려고 R >= B)
    semi = (lum > 100) & (lum < 185) & (sat < 80) & (rgb[..., 0] >= rgb[..., 2] - 5)
    near_dark = cv2.dilate(dark(im).astype(np.uint8), np.ones((5, 5), np.uint8)) > 0
    g = semi & ~near_dark
    # 한 점짜리 잡티는 빼고 선 모양만
    g = cv2.morphologyEx(g.astype(np.uint8), cv2.MORPH_OPEN, np.ones((2, 2), np.uint8)) > 0
    return g


def n_comp(m: np.ndarray, min_area: int = 20) -> int:
    n, _, st, _ = cv2.connectedComponentsWithStats(m.astype(np.uint8), 8)
    return int((st[1:, cv2.CC_STAT_AREA] >= min_area).sum())


def resize_mask(m: np.ndarray, size: int) -> np.ndarray:
    return cv2.resize(m.astype(np.float32), (size, size), interpolation=cv2.INTER_AREA)


class Char:
    def __init__(self, name: str, size: int):
        self.name = name
        self.dir = ROOT / "characters" / name
        self.m = tomllib.loads((self.dir / "manifest.toml").read_text())
        L = self.dir / "layers"
        self.base = wf.load(L / "base.png")
        self.clean = wf.load(L / "base_clean.png")
        self.canvas = self.base.shape[0]
        self.warp = wf.load(L / "mask/warp.png")[..., :3] / 255.0
        self.size = size
        self.k = size / self.canvas
        # 렌더 크기의 기준 마스크
        clean_r = cv2.resize(self.clean, (size, size), interpolation=cv2.INTER_AREA)
        base_r = cv2.resize(self.base, (size, size), interpolation=cv2.INTER_AREA)
        self.desk_lines = dark(clean_r)
        char = (base_r[..., 3] > 128) & ~(clean_r[..., 3] > 128) | (np.abs(flat(base_r) - flat(clean_r)).max(axis=-1) > 40)
        self.char_near = cv2.dilate(char.astype(np.uint8), np.ones((int(48 * self.k) | 1,) * 2, np.uint8)) > 0
        self.w_r = cv2.resize(self.warp.astype(np.float32), (size, size), interpolation=cv2.INTER_AREA)
        self.base_r = base_r
        self.desk_r = clean_r[..., 3] > 128
        # 마우스 리그: 손+마우스 (한 덩어리로 움직이는 곳), 팔까지 포함한 자리 (렌더 크기)
        self.rig = self.m.get("mouse")
        self.rig_core = self.rig_all = None
        if self.rig and (L / "fx/mouse.png").exists() and (L / "fx/mouse_arm.png").exists():
            ma = cv2.resize(wf.load(L / "fx/mouse.png")[..., 3], (size, size), interpolation=cv2.INTER_AREA) / 255.0
            aa = cv2.resize(wf.load(L / "fx/mouse_arm.png")[..., 3], (size, size), interpolation=cv2.INTER_AREA) / 255.0
            root, tip = np.array(self.rig["root"]) * self.k, np.array(self.rig["tip"]) * self.k
            ax = tip - root
            yy, xx = np.mgrid[:size, :size]
            s_ax = ((xx - root[0]) * ax[0] + (yy - root[1]) * ax[1]) / (ax @ ax)
            hand = (aa > 0.5) & (s_ax >= self.rig.get("hand", 0.72) + 0.05)
            self.rig_core = cv2.erode(((ma > 0.5) | hand).astype(np.uint8), np.ones((3, 3), np.uint8)) > 0
            self.rig_all = (ma > 0.02) | (aa > 0.02)


def jacobian_range(W: np.ndarray, defs: list) -> tuple[float, float]:
    """셰이더와 같은 사상 suv = x + Wb (Pb(x) - x) + Wh (Ph(x) - x) - Wm m 의 야코비안 행렬식 최소/최대 (256 격자)."""
    n = 256
    k = n / W.shape[0]
    w = cv2.resize(W.astype(np.float32), (n, n), interpolation=cv2.INTER_AREA)
    yy, xx = np.mgrid[:n, :n].astype(np.float32) / k
    lo, hi = 1.0, 1.0
    for d in defs:
        def inv(a):
            ax, ay = a["anchor"]
            return ax + (xx - ax - a["offset"][0]) / a["s"][0], ay + (yy - ay - a["offset"][1]) / a["s"][1]
        bx, by = inv(d["body"])
        hx, hy = inv(d["head"])
        sx = xx + w[..., 0] * (bx - xx) + w[..., 1] * (hx - xx)
        sy = yy + w[..., 0] * (by - yy) + w[..., 1] * (hy - yy)
        gxy, gxx = np.gradient(sx * k)
        gyy, gyx = np.gradient(sy * k)
        det = gxx * gyy - gxy * gyx
        lo, hi = min(lo, float(det.min())), max(hi, float(det.max()))
    return lo, hi


def warp_mask(mask: np.ndarray, c: "Char", d: dict | None) -> np.ndarray:
    """렌더 크기 마스크를 셰이더 사상 suv = x + Wb (Pb(x) - x) + Wh (Ph(x) - x) - Wm m 으로 옮긴다."""
    return warp_img(mask.astype(np.uint8), c, d, cv2.INTER_NEAREST) > 0 if d else mask


def warp_img(img: np.ndarray, c: "Char", d: dict | None, interp: int = cv2.INTER_LINEAR) -> np.ndarray:
    if not d:
        return img
    yy, xx = np.mgrid[:c.size, :c.size].astype(np.float32)
    X, Y = xx / c.k, yy / c.k

    def inv(a):
        ax, ay = a["anchor"]
        return ax + (X - ax - a["offset"][0]) / a["s"][0], ay + (Y - ay - a["offset"][1]) / a["s"][1]

    w = c.w_r
    bx, by = inv(d["body"])
    hx, hy = inv(d["head"])
    sx = X + w[..., 0] * (bx - X) + w[..., 1] * (hx - X)
    sy = Y + w[..., 0] * (by - Y) + w[..., 1] * (hy - Y)
    return cv2.remap(img, (sx * c.k).astype(np.float32), (sy * c.k).astype(np.float32), interp, borderMode=cv2.BORDER_REPLICATE)


def check_field(c: Char, dmax: dict, defs: list) -> list[dict]:
    """F1~F3. dmax: 채널별 관측 최대 변위 (캔버스 px)."""
    res = []
    W = c.warp
    s = c.canvas / 1024
    desk_dark = dark(c.clean)
    desk = c.clean[..., 3] > 128
    # F1: 책상 윤곽 위 몸/머리 가중치
    v = float(max(W[..., 0][desk_dark].max(), W[..., 1][desk_dark].max()))
    res.append(dict(id="F1", item="책상 고정 (몸/머리)", value=round(v, 4), limit="< 0.02", ok=v < 0.02))
    # F2: 접힘. 관측된 변형 중 채널별로 큰 것들의 실제 샘플링 사상 x -> suv(x) 의 야코비안 행렬식 범위
    for key, label in (("body", "몸"), ("head", "머리")):
        ds = sorted(defs, key=lambda d: -deform_disp(d, c.canvas)[key])[:40]
        lo, hi = jacobian_range(W, ds) if ds else (1.0, 1.0)
        d = dmax.get(key, 0.0)
        ok = lo > FOLD_DET[0] and hi < FOLD_DET[1]
        res.append(dict(id="F2", item=f"접힘 없음 ({label}, 최대 변위 {d:.1f}px, 행렬식 최소/최대)", value=f"{lo:.2f} / {hi:.2f}", limit=f"{FOLD_DET[0]} ~ {FOLD_DET[1]}", ok=ok))
    # F3: 캐릭터 전체 이동
    char = (c.base[..., 3] > 128) & ~desk
    far = wf.dist_to(desk) > 100 * s
    v = float(W[..., 0][char & far].min()) if (char & far).any() else 1.0
    res.append(dict(id="F3", item="캐릭터 전체가 같이 움직임 (책상에서 100px 밖 몸 가중치 최소)", value=round(v, 4), limit="> 0.98", ok=v > 0.98))
    return res


def deform_disp(d: dict | None, canvas: int) -> dict:
    """프레임 변형의 채널별 최대 변위 (캔버스 px, 보수적으로 기준점에서 600px 떨어진 곳의 배율 효과 포함)."""
    if not d:
        return {"body": 0.0, "head": 0.0}
    R = 600 * canvas / 1024

    def one(a):
        return float(np.hypot(*a["offset"]) + max(abs(a["s"][0] - 1), abs(a["s"][1] - 1)) * R)
    return {"body": one(d["body"]), "head": one(d["head"])}


def frame_dirs(root: Path) -> list[tuple[str, Path, list]]:
    out = []
    for d in sorted((root / "bhv").glob("*")):
        if (d / "frames.json").exists():
            out.append((f"bhv:{d.name}", d, json.loads((d / "frames.json").read_text())))
    for d in sorted(root.glob("tl_*")):
        if (d / "timeline.json").exists():
            log = json.loads((d / "timeline.json").read_text())
            files = sorted(p for p in d.glob("[0-9]*.png") if p.stem.isdigit())
            for e, p in zip(log, files):
                e["file"] = p.name
            out.append((d.name, d, log))
    return out


def check_renders(c: Char, root: Path) -> tuple[list[dict], dict, list]:
    """R1~R4 와 관측 최대 변위, 시트에 넣을 (점수, 경로) 목록."""
    dmax = {"body": 0.0, "head": 0.0}
    defs = []
    worst = {"R1": (0.0, ""), "R1b": (0.0, ""), "R2": (0.0, ""), "R3": (1.0, ""), "R4": (0, ""), "M4": (0.0, "")}
    n_pairs = 0
    n_mouse = 0
    sheet = []
    lines_ok = c.desk_lines & ~c.char_near & (c.w_r[..., 0] < 0.02) & (c.w_r[..., 1] < 0.02)
    if c.rig_all is not None:
        # 손/팔/마우스가 지나가는 자리는 M4 가 따로 본다
        lines_ok &= ~(cv2.dilate(c.rig_all.astype(np.uint8), np.ones((int(80 * c.k) | 1,) * 2, np.uint8)) > 0)
    base_desk = flat(c.base_r)
    mouse_core = c.rig_core if c.rig_core is not None else np.zeros((c.size, c.size), bool)
    for label, d, log in frame_dirs(root):
        for i, e in enumerate(log):
            f = d / e.get("file", f"{e.get('i', i):05d}.png")
            if not f.exists():
                continue
            img = None
            dd = deform_disp(e.get("deform"), c.canvas)
            if e.get("deform"):
                defs.append(e["deform"])
            for k in dmax:
                dmax[k] = max(dmax[k], dd[k])
            s = f.with_name(f.stem + "_s.png")
            if not s.exists():
                continue
            n_pairs += 1
            img = rgba(f) if img is None else img
            st = rgba(s)
            # R1: 변형이 책상을 건드리지 않음
            v = float(np.abs(flat(img) - flat(st))[lines_ok].mean())
            if v > worst["R1"][0]:
                worst["R1"] = (v, f"{label}/{f.name}")
            if not e.get("mix"):
                # R2: 반투명 선, R4: 진한 선 덩어리 수
                g = (ghost(img).sum() - ghost(st).sum()) / img[..., 0].size
                if g > worst["R2"][0]:
                    worst["R2"] = (float(g), f"{label}/{f.name}")
                dc = n_comp(dark(img)) - n_comp(dark(st))
                if dc > worst["R4"][0]:
                    worst["R4"] = (dc, f"{label}/{f.name}")
            # R3: 손+마우스가 한 덩어리로
            m = e.get("hand") or [0, 0]
            if np.hypot(*m) * c.k > 0.5 and c.rig_all is not None:
                # M4: 손/팔/마우스가 지나간 자리 밖의 책상은 정지 프레임과 같다
                r = int((np.hypot(*m) + 8) * c.k) * 2 + 1
                keep = c.desk_r & ~(cv2.dilate(c.rig_all.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (r, r))) > 0)
                x = float((np.abs(flat(img) - flat(st)).max(axis=-1)[keep] > 30).mean())
                if x > worst["M4"][0]:
                    worst["M4"] = (x, f"{label}/{f.name}")
            if np.hypot(*m) * c.k > 1.5 and mouse_core.any():
                n_mouse += 1
                sh = np.float32([[1, 0, m[0] * c.k], [0, 1, m[1] * c.k]])
                moved = cv2.warpAffine(flat(st), sh, (c.size, c.size), flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_REPLICATE)
                core = cv2.warpAffine(mouse_core.astype(np.uint8), sh, (c.size, c.size)) > 0
                a, b = flat(img)[core].ravel(), moved[core].ravel()
                ncc = float(np.corrcoef(a, b)[0, 1]) if a.std() > 0 and b.std() > 0 else 1.0
                if ncc < worst["R3"][0]:
                    worst["R3"] = (ncc, f"{label}/{f.name}")
            sheet.append((max(max(dd.values()), float(np.hypot(*m))), label, f, s))
    # R1b: 장면/behavior 전체 프레임 레이어의 책상 선이 base 와 같은 자리 (장면이 시작될 때 책상이 튀지 않음).
    # 캐릭터 팔이나 소품이 가린 선이 있어서 선 픽셀 차이의 중앙값으로 본다.
    for lf in sorted((c.dir / "layers/full").glob("*.png")):
        im = rgba(lf, c.size)
        v = float(np.median(np.abs(flat(im) - base_desk).max(axis=-1)[lines_ok]))
        if v > worst["R1b"][0]:
            worst["R1b"] = (v, f"full/{lf.stem}")
    res = [
        dict(id="R1", item=f"변형이 책상 선을 건드리지 않음 (짝 {n_pairs}장)", value=round(worst["R1"][0], 3), limit=f"< {DESK_DIFF}", ok=worst["R1"][0] < DESK_DIFF, at=worst["R1"][1]),
        dict(id="R1b", item="모든 프레임의 책상 선이 base 와 같은 자리", value=round(worst["R1b"][0], 2), limit=f"< {SCENE_DESK_DIFF}", ok=worst["R1b"][0] < SCENE_DESK_DIFF, at=worst["R1b"][1]),
        dict(id="R2", item="반투명 선(고스트) 증가", value=round(worst["R2"][0] * 100, 4), limit=f"<= {GHOST_FRAC * 100}%", ok=worst["R2"][0] <= GHOST_FRAC, at=worst["R2"][1]),
        dict(id="R3", item=f"손+마우스 강체 NCC (마우스 프레임 {n_mouse}장)", value=round(worst["R3"][0], 3), limit=f"> {NCC_MIN}", ok=worst["R3"][0] > NCC_MIN and n_mouse > 0, at=worst["R3"][1]),
        dict(id="R4", item="진한 선 덩어리 증가 (끊김/겹침)", value=worst["R4"][0], limit=f"<= {COMP_PLUS}", ok=worst["R4"][0] <= COMP_PLUS, at=worst["R4"][1]),
    ]
    if c.rig_all is not None:
        res.append(dict(id="M4", item="마우스를 움직여도 손/팔/마우스 밖 책상은 그대로 (달라진 픽셀 비율)", value=round(worst["M4"][0] * 100, 4), limit=f"<= {DESK_MOVED * 100}%", ok=worst["M4"][0] <= DESK_MOVED, at=worst["M4"][1]))
    return res, dmax, sheet, defs


# ---------- T: 움직임 수정 계획 기준 (docs/MOTION_FIX_PLAN.md 4장 1단계) ----------


def char_mask(im: np.ndarray, clean: np.ndarray) -> np.ndarray:
    """책상(base_clean) 과 다른 캐릭터 픽셀."""
    return (np.abs(flat(im) - flat(clean)).max(axis=-1) > 40) | ((im[..., 3] > 128) & ~(clean[..., 3] > 128))


class Disp:
    """변형 하나가 캐릭터 픽셀을 미는 최대 거리 (몸/머리 채널, 1024 캔버스 px). 마우스는 [mouse] range 가 따로 막는다."""

    def __init__(self, c: "Char"):
        n = 128
        self.k = n / c.canvas
        self.unit = 1024 / c.canvas
        w = cv2.resize(c.warp.astype(np.float32), (n, n), interpolation=cv2.INTER_AREA)
        m = cv2.resize(char_mask(c.base, c.clean).astype(np.float32), (n, n), interpolation=cv2.INTER_AREA) > 0.3
        yy, xx = np.mgrid[:n, :n].astype(np.float32) / self.k
        self.x, self.y = xx[m], yy[m]
        self.wb, self.wh = w[..., 0][m], w[..., 1][m]

    def __call__(self, d: dict | None) -> float:
        if not d or self.x.size == 0:
            return 0.0
        dx = np.zeros_like(self.x)
        dy = np.zeros_like(self.y)
        for a, wt in ((d["body"], self.wb), (d["head"], self.wh)):
            ax, ay = a["anchor"]
            # 앞으로 미는 변환 p -> anchor + (p - anchor) s + offset 의 변위
            dx += wt * ((self.x - ax) * (a["s"][0] - 1) + a["offset"][0])
            dy += wt * ((self.y - ay) * (a["s"][1] - 1) + a["offset"][1])
        return float(np.hypot(dx, dy).max() * self.unit)


def motion_behaviors(m: dict) -> tuple[dict, list]:
    """behavior 이름 -> 몸/머리 motion 목록, 그리고 아무것도 움직이지 않는 overlay behavior (motion, fx, 머리 교대 없음)."""
    moving, still = {}, []
    for b in m.get("behavior", []):
        ms = [x for x in b.get("motion", []) if x.get("target") in ("body", "head")]
        if ms:
            moving[b["name"]] = ms
        full = b.get("layer_mode") == "full" or b.get("frames") or b.get("step")
        head = b.get("head") or {}
        if not full and not b.get("motion") and not b.get("fx") and len(head.get("frames", [])) < 2:
            still.append(b["name"])
    return moving, still


def check_t_renders(c: "Char", root: Path) -> tuple[list[dict], dict]:
    """T1 (섞인 프레임 고스트), T3 (변위 상한), T4 (키 빛이 앞발을 물들이지 않음), T6 (움직임 보임). 그리고 behavior 별 최대 변위."""
    disp = Disp(c)
    worst = {"T1": (0.0, ""), "T3": (0.0, ""), "T4": (0.0, "")}
    n_mix = n_glow = 0
    bmax: dict[str, float] = {}
    paw = np.zeros((c.size, c.size), bool)
    for g in ("paw_l", "paw_r"):
        mp = c.dir / "layers" / f"mask/{g}.png"
        if mp.exists():
            paw |= cv2.resize(wf.load(mp)[..., 0] / 255.0, (c.size, c.size), interpolation=cv2.INTER_AREA) > 0.5
    clean_r = flat(cv2.resize(c.clean, (c.size, c.size), interpolation=cv2.INTER_AREA))
    for label, d, log in frame_dirs(root):
        for i, e in enumerate(log):
            f = d / e.get("file", f"{e.get('i', i):05d}.png")
            if not f.exists():
                continue
            v = disp(e.get("deform"))
            bmax[label] = max(bmax.get(label, 0.0), v)
            if v > worst["T3"][0]:
                worst["T3"] = (v, f"{label}/{f.name}")
            u = f.with_name(f.stem + "_u.png")
            g = f.with_name(f.stem + "_g.png")
            img = rgba(f) if u.exists() or g.exists() else None
            if u.exists():
                n_mix += 1
                x = (ghost(img).sum() - ghost(rgba(u)).sum()) / img[..., 0].size
                if x > worst["T1"][0]:
                    worst["T1"] = (float(x), f"{label}/{f.name}")
            if g.exists():
                n_glow += 1
                ng = rgba(g)
                D = np.abs(flat(img) - flat(ng)).max(axis=-1)
                near = cv2.dilate((D > 3).astype(np.uint8), np.ones((15, 15), np.uint8)) > 0
                # 변형이 있으면 앞발 마스크도 셰이더와 같은 사상으로 옮긴다 (마우스 흔들기, 클릭 눌림, 몸 기울기)
                pw = warp_mask(paw, c, e.get("deform"))
                # 빈 책상도 같은 변형으로 (패드와 마우스가 손과 같이 움직인다)
                cr = warp_img(clean_r.astype(np.float32), c, e.get("deform"))
                P = near & pw & (np.abs(flat(ng) - cr).max(axis=-1) > 40)
                x = float(D[P].mean()) if P.sum() > 20 else 0.0
                if x > worst["T4"][0]:
                    worst["T4"] = (x, f"{label}/{f.name}")
    res = [
        dict(id="T1", item=f"섞인 프레임의 고스트 증가 (섞인 프레임 {n_mix}장, 한 장으로 고른 기준과 비교)", value=round(worst["T1"][0] * 100, 4), limit=f"<= {MIX_GHOST * 100}%", ok=worst["T1"][0] <= MIX_GHOST, at=worst["T1"][1]),
        dict(id="T3", item="몸/머리 움직임의 최대 변위 (1024 캔버스 px)", value=round(worst["T3"][0], 1), limit=f"<= {DISP_MAX}", ok=worst["T3"][0] <= DISP_MAX, at=worst["T3"][1]),
        dict(id="T4", item=f"키 빛 근처 앞발 색 차이 평균 (빛 프레임 {n_glow}장)", value=round(worst["T4"][0], 2), limit=f"< {GLOW_PAW}", ok=worst["T4"][0] < GLOW_PAW and n_glow > 0, at=worst["T4"][1]),
    ]
    moving, still = motion_behaviors(c.m)
    weak = sorted((round(bmax.get(f"bhv:{n}", 0.0), 1), n) for n in moving if bmax.get(f"bhv:{n}", 0.0) < DISP_MIN)
    fast = sorted({b for b, ms in moving.items() for x in ms if x["kind"] == "tremble" and x.get("period_ms", MOTION_DEFAULTS["tremble"][1]) < TREMBLE_MIN_MS})
    bad = [f"{n}({v}px)" for v, n in weak] + [f"{n}(움직임 없음)" for n in still] + [f"{n}(떨림 주기)" for n in fast]
    res.append(dict(id="T6", item=f"움직임이 보임 (최대 변위 >= {DISP_MIN}px, 떨림 주기 >= {TREMBLE_MIN_MS}ms, 움직임 없는 behavior 없음)", value=len(bad), limit="0", ok=not bad, at=", ".join(bad)))
    return res, bmax


def full_groups(m: dict) -> list[tuple[str, list[str]]]:
    """여러 장을 교대하는 behavior/장면과 그 프레임들."""
    out = []
    for sec in ("behavior", "scene"):
        for b in m.get(sec, []):
            fr = list(dict.fromkeys(b.get("frames") or [s["frame"] for s in b.get("step", [])]))
            if len(fr) > 1:
                out.append((b["name"], fr))
    return out


def check_layers(c: "Char") -> list[dict]:
    """T2 (a/b 정합), T5 (마우스 팔 길이). 레이어 그림만 본다."""
    L = c.dir / "layers"
    res = []
    # T2: 차이 덩어리(두꺼운 것, 움직이는 부위)를 넓힌 영역 밖의 진한 선이 서로 2px 안에 있어야 한다
    worst = (0.0, "")
    for name, fr in full_groups(c.m):
        ims = [wf.load(L / f"{f}.png") for f in fr if (L / f"{f}.png").exists()]
        s = c.canvas / 1024
        for i in range(len(ims)):
            for j in range(i + 1, len(ims)):
                a, b = ims[i], ims[j]
                D = (np.abs(flat(a) - flat(b)).max(axis=-1) > 40).astype(np.uint8)
                k = int(9 * s) | 1
                mov = cv2.morphologyEx(D, cv2.MORPH_OPEN, np.ones((k, k), np.uint8))
                da, db = dark(a), dark(b)
                ta, tb = wf.dist_to(db), wf.dist_to(da)
                # 가는 소품(지팡이 등)이 다른 자리로 옮겨 간 선: 다른 그림에 LONE_PX 안에 짝 선이 없다 -> 움직이는 부위
                lone = (da & (ta > LONE_PX * s)) | (db & (tb > LONE_PX * s))
                lone = cv2.morphologyEx(lone.astype(np.uint8), cv2.MORPH_OPEN, np.ones((2, 2), np.uint8))
                movd = cv2.dilate(mov | lone, np.ones((int(31 * s) | 1,) * 2, np.uint8)) > 0
                dist = np.concatenate([ta[da & ~movd], tb[db & ~movd]]) / s
                v = float(np.percentile(dist, 95)) if dist.size else 0.0
                if v > worst[0]:
                    worst = (v, f"{name}: {fr[i]} / {fr[j]}")
    res.append(dict(id="T2", item="a/b 정합 (움직이는 부위 밖 윤곽선 거리 95 백분위, px)", value=round(worst[0], 1), limit=f"< {PAIR_LINE_PX}", ok=worst[0] < PAIR_LINE_PX, at=worst[1]))
    # T5: 팔 길이 = 어깨(root)에서 손끝까지. 쉬는 앞발(발끝에서 앞발 윤곽선까지 가장 먼 거리)의 2.5배 안.
    # 몸 한가운데에서 길게 뻗은 팔을 막는다 (예전 생성 그림은 3배가 넘었다).
    pm_p = L / "mask/paw_r.png"
    tips = c.m.get("tips", {})
    if pm_p.exists() and "paw_r/down" in tips and c.rig:
        pm = wf.load(pm_p)[..., 0] / 255.0
        tip = tips["paw_r/down"]
        paw = dark(c.base) & ~dark(c.clean) & (pm > 0.5)
        yy, xx = np.mgrid[:c.canvas, :c.canvas]
        paw &= np.hypot(xx - tip[0], yy - tip[1] + 30 * c.canvas / 1024) < 70 * c.canvas / 1024
        ys, xs = np.nonzero(paw)
        rest = float(np.percentile(np.hypot(xs - tip[0], ys - tip[1]), 99)) if xs.size else 0.0
        arm = float(np.hypot(c.rig["tip"][0] - c.rig["root"][0], c.rig["tip"][1] - c.rig["root"][1]))
        r = arm / rest if rest else 99.0
        res.append(dict(id="T5", item="마우스 팔 길이 / 쉬는 앞발 길이", value=round(r, 2), limit=f"<= {ARM_RATIO}", ok=r <= ARM_RATIO, at=f"팔 {arm:.0f}px / 앞발 {rest:.0f}px"))
    return res


def make_sheet_all(root: Path, out: Path, n: int = 8, tile: int = 160) -> None:
    """behavior 마다 고르게 n 장을 한 줄로 (한 캐릭터의 모든 behavior 를 한 장으로, docs/MOTION_FIX_PLAN.md 6장)."""
    from PIL import ImageDraw
    rows = []
    for d in sorted((root / "bhv").glob("*")):
        files = sorted(p for p in d.glob("[0-9]*.png") if p.stem.isdigit())
        if not files:
            continue
        pick = [files[round(k * (len(files) - 1) / max(n - 1, 1))] for k in range(n)]
        row = Image.new("RGB", (tile * n + 140, tile), "white")
        ImageDraw.Draw(row).text((4, tile // 2 - 6), d.name, fill="black")
        for k, p in enumerate(pick):
            im = Image.open(p).convert("RGBA")
            bg = Image.new("RGB", im.size, "white")
            bg.paste(im, mask=im)
            row.paste(bg.resize((tile, tile)), (140 + k * tile, 0))
        rows.append(row)
    if not rows:
        return
    S = Image.new("RGB", (rows[0].width, tile * len(rows)), "white")
    for i, r in enumerate(rows):
        S.paste(r, (0, i * tile))
    S.save(out)


def make_sheet(sheet: list, out: Path, size: int) -> None:
    """behavior/시나리오마다 변위가 가장 큰 프레임: 움직인 것 | 정지."""
    best: dict[str, tuple] = {}
    for sc, label, f, s in sheet:
        if label not in best or sc > best[label][0]:
            best[label] = (sc, f, s)
    tiles = []
    for label, (sc, f, s) in sorted(best.items()):
        a, b = Image.open(f).convert("RGB"), Image.open(s).convert("RGB")
        t = Image.new("RGB", (size * 2, size + 18), "white")
        t.paste(a, (0, 18))
        t.paste(b, (size, 18))
        from PIL import ImageDraw
        ImageDraw.Draw(t).text((4, 2), f"{label} {f.name} max {sc:.1f}px | 정지", fill="black")
        tiles.append(t)
    if not tiles:
        return
    cols = 2
    rows = (len(tiles) + cols - 1) // cols
    S = Image.new("RGB", (tiles[0].width * cols, tiles[0].height * rows), "white")
    for i, t in enumerate(tiles):
        S.paste(t, ((i % cols) * t.width, (i // cols) * t.height))
    S.save(out)


def check_mouse(c: "Char", root: Path) -> list[dict]:
    """M1~M3: mouse 시나리오에서 손이 크게, 커서 방향으로, 멈추면 한 번 넘쳤다 돌아오는가 (docs/MOTION_QA.md)."""
    p = root / "tl_mouse" / "timeline.json"
    if not p.exists():
        return []
    if c.rig_all is None:
        # 마우스 자세가 있는데 리그가 없으면 손이 움직이지 않는다
        has_pose = (c.dir / "layers/paw_r/mouse.png").exists()
        return [dict(id="M1", item="마우스 리그 ([mouse], fx/mouse, fx/mouse_arm)", value="없음", limit="있어야 함", ok=False)] if has_pose else []
    log = json.loads(p.read_text())
    k = 1024 / c.canvas
    mx = np.array([(e.get("hand") or [0, 0])[0] * k for e in log])
    my = np.array([(e.get("hand") or [0, 0])[1] * k for e in log])
    cur = np.array([(e.get("cursor") or [0, 0])[0] for e in log], dtype=float)
    res = []
    peak = float(np.hypot(mx, my).max()) if len(log) else 0.0
    at = int(np.hypot(mx, my).argmax()) if len(log) else 0
    res.append(dict(id="M1", item="마우스 흔들 때 손+마우스 최대 변위 (1024 px)", value=round(peak, 2), limit=f">= {MOUSE_MIN_PX}", ok=peak >= MOUSE_MIN_PX, at=f"tl_mouse/{at:05d}.png"))
    moving = np.nonzero(cur != 0)[0]
    if len(moving) == 0:
        res.append(dict(id="M2", item="커서가 멈추면 반대쪽으로 한 번 넘쳤다 돌아옴", value="방향 있는 이동 없음", limit="", ok=False))
        res.append(dict(id="M3", item="손이 커서 방향을 따라감", value="방향 있는 이동 없음", limit="", ok=False))
        return res
    stop = int(moving[-1])
    ms = [e["ms"] for e in log]
    x0 = float(mx[max(stop - 1, 0):stop + 2][np.argmax(np.abs(mx[max(stop - 1, 0):stop + 2]))])
    after = [i for i in range(stop + 1, len(log)) if ms[i] - ms[stop] <= 600]
    opp = [float(-np.sign(x0) * mx[i]) for i in after]
    ov = max(opp) / abs(x0) if opp and abs(x0) > 0.5 else 0.0
    res.append(dict(id="M2", item=f"커서가 멈추면 반대쪽으로 넘쳤다 돌아옴 (멈출 때 {x0:.1f}px 대비)", value=round(ov, 3), limit=f">= {OVERSHOOT_MIN}", ok=ov >= OVERSHOOT_MIN, at=f"tl_mouse/{stop:05d}.png"))
    lo, hi = int(moving[0]), stop + 1
    best = -1.0
    for lag in range(0, 3):
        a, b = np.sign(cur[lo:hi]), mx[lo + lag:hi + lag]
        n = min(len(a), len(b))
        a, b = a[:n], b[:n]
        if n > 3 and a.std() > 0 and b.std() > 0:
            best = max(best, float(np.corrcoef(a, b)[0, 1]))
    res.append(dict(id="M3", item="손이 커서 방향을 따라감 (커서 방향과 손 가로 변위 상관)", value=round(best, 3), limit=f">= {FOLLOW_MIN}", ok=best >= FOLLOW_MIN))
    return res


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "target/motion_qa"))
    ap.add_argument("--size", type=int, default=512)
    ap.add_argument("names", nargs="+")
    a = ap.parse_args()
    out = Path(a.out).resolve()
    report = {}
    lines = ["# 움직임 QA 결과", "", "기준: docs/MOTION_QA.md", ""]
    all_ok = True
    for name in a.names:
        c = Char(name, a.size)
        rres, dmax, sheet, defs = check_renders(c, out / name)
        fres = check_field(c, dmax, defs)
        tres, _ = check_t_renders(c, out / name)
        tres = sorted(tres + check_layers(c) + check_mouse(c, out / name), key=lambda r: r["id"])
        make_sheet(sheet, out / name / "sheet_motion.png", a.size)
        make_sheet_all(out / name, out / name / "sheet_all.png")
        rows = fres + rres + tres
        ok = all(r["ok"] for r in rows)
        all_ok &= ok
        report[name] = rows
        lines += [f"## {name}: {'PASS' if ok else 'FAIL'}", "", "| id | 항목 | 값 | 기준 | 결과 | 최악 프레임 |", "|---|---|---|---|---|---|"]
        for r in rows:
            lines.append(f"| {r['id']} | {r['item']} | {r['value']} | {r['limit']} | {'PASS' if r['ok'] else 'FAIL'} | {r.get('at', '')} |")
        lines += ["", f"눈 확인: `{(out / name / 'sheet_motion.png').relative_to(ROOT)}`", ""]
        print(f"[{'PASS' if ok else 'FAIL'}] {name}: " + ", ".join(f"{r['id']}={r['value']}" for r in rows if not r["ok"]))
    lines.insert(3, f"**전체: {'PASS' if all_ok else 'FAIL'}**\n")
    (out / "report.md").write_text("\n".join(lines))
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=1))
    sys.exit(0 if all_ok else 1)


if __name__ == "__main__":
    main()
