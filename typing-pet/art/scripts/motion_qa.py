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
SWING_PT = [2.0, 0.25]  # src/config.rs Anim::default swing_pt
FOLD_DET = (0.5, 2.0)  # F2: 샘플링 사상의 야코비안 행렬식 범위 (0 이하면 접힘, 0.5 미만/2 초과면 두 배 넘게 늘어나거나 눌림)


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
        sx = xx + w[..., 0] * (bx - xx) + w[..., 1] * (hx - xx) - w[..., 2] * d["mouse"][0]
        sy = yy + w[..., 0] * (by - yy) + w[..., 1] * (hy - yy) - w[..., 2] * d["mouse"][1]
        gxy, gxx = np.gradient(sx * k)
        gyy, gyx = np.gradient(sy * k)
        det = gxx * gyy - gxy * gyx
        lo, hi = min(lo, float(det.min())), max(hi, float(det.max()))
    return lo, hi


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
    # F1m: 마우스 덩어리에서 40px 넘게 떨어진 책상 선의 마우스 가중치
    mouse = W[..., 2] > 0.98
    if mouse.any():
        far = wf.dist_to(mouse) > 40 * s
        # 마우스 자세에서 실제로 보이는 책상 선만 (팔에 가려진 선은 팔과 같이 움직여도 안 보인다)
        L = c.dir / "layers"
        comp = wf.over_mask(c.base, wf.load(L / "paw_r/mouse.png"), wf.load(L / "mask/paw_r.png")[..., 0] / 255.0)
        visible = desk_dark & dark(comp)
        # 선을 따라 미는 것은 안 보이므로 선에 수직인 변위만 본다 (선 법선은 흐린 선 마스크의 기울기)
        lb = cv2.GaussianBlur(desk_dark.astype(np.float32), (0, 0), 2.0 * s)
        gx, gy = cv2.Sobel(lb, cv2.CV_32F, 1, 0, ksize=5), cv2.Sobel(lb, cv2.CV_32F, 0, 1, ksize=5)
        gx, gy = cv2.GaussianBlur(gx * np.sign(gx), (0, 0), 3 * s), cv2.GaussianBlur(gy * np.sign(gy), (0, 0), 3 * s)
        nrm = np.hypot(gx, gy) + 1e-6
        # 실제 진폭: min(swing_pt x 캔버스/240, max_px x 0.9) (src/anim.rs mouse_offset)
        swing_pt = c.m.get("anim", {}).get("swing_pt", SWING_PT)
        mx, my = [min(p * c.canvas / 240, 0.9 * v) for p, v in zip(swing_pt, c.m.get("swing", {}).get("max_px", [0, 0]))]
        perp = W[..., 2] * (mx * gx / nrm + my * gy / nrm)
        v = float(perp[visible & far].max())
        res.append(dict(id="F1m", item="책상 선이 마우스 따라 밀리지 않음 (손에서 40px 밖, 선에 수직 변위 px)", value=round(v, 3), limit="< 0.5", ok=v < 0.5))
    # F2: 접힘. 관측된 변형 중 채널별로 큰 것들의 실제 샘플링 사상 x -> suv(x) 의 야코비안 행렬식 범위
    for key, label in (("body", "몸"), ("head", "머리"), ("mouse", "마우스")):
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
    if mouse.any():
        core = cv2.erode(mouse.astype(np.uint8), np.ones((3, 3), np.uint8)) > 0
        area = int(core.sum())
        res.append(dict(id="F3m", item="손+마우스 강체 영역 넓이", value=area, limit=f"> {int(1500 * s * s)}px", ok=area > 1500 * s * s))
    return res


def deform_disp(d: dict | None, canvas: int) -> dict:
    """프레임 변형의 채널별 최대 변위 (캔버스 px, 보수적으로 기준점에서 600px 떨어진 곳의 배율 효과 포함)."""
    if not d:
        return {"body": 0.0, "head": 0.0, "mouse": 0.0}
    R = 600 * canvas / 1024

    def one(a):
        return float(np.hypot(*a["offset"]) + max(abs(a["s"][0] - 1), abs(a["s"][1] - 1)) * R)
    return {"body": one(d["body"]), "head": one(d["head"]), "mouse": float(np.hypot(*d["mouse"]))}


def frame_dirs(root: Path) -> list[tuple[str, Path, list]]:
    out = []
    for d in sorted((root / "bhv").glob("*")):
        if (d / "frames.json").exists():
            out.append((f"bhv:{d.name}", d, json.loads((d / "frames.json").read_text())))
    for d in sorted(root.glob("tl_*")):
        if (d / "timeline.json").exists():
            log = json.loads((d / "timeline.json").read_text())
            files = sorted(p for p in d.glob("[0-9]*.png") if not p.stem.endswith("_s"))
            for e, p in zip(log, files):
                e["file"] = p.name
            out.append((d.name, d, log))
    return out


def check_renders(c: Char, root: Path) -> tuple[list[dict], dict, list]:
    """R1~R4 와 관측 최대 변위, 시트에 넣을 (점수, 경로) 목록."""
    dmax = {"body": 0.0, "head": 0.0, "mouse": 0.0}
    defs = []
    worst = {"R1": (0.0, ""), "R1b": (0.0, ""), "R2": (0.0, ""), "R3": (1.0, ""), "R4": (0, "")}
    n_pairs = 0
    n_mouse = 0
    sheet = []
    lines_ok = c.desk_lines & ~c.char_near & (c.w_r[..., 0] < 0.02) & (c.w_r[..., 1] < 0.02) & (c.w_r[..., 2] < 0.02)
    base_desk = flat(c.base_r)
    mouse_core = cv2.erode((c.w_r[..., 2] > 0.98).astype(np.uint8), np.ones((3, 3), np.uint8)) > 0
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
            m = (e.get("deform") or {}).get("mouse", [0, 0])
            if np.hypot(*m) * c.k > 1.5 and mouse_core.any():
                n_mouse += 1
                sh = np.float32([[1, 0, m[0] * c.k], [0, 1, m[1] * c.k]])
                moved = cv2.warpAffine(flat(st), sh, (c.size, c.size), flags=cv2.INTER_LINEAR, borderMode=cv2.BORDER_REPLICATE)
                core = cv2.warpAffine(mouse_core.astype(np.uint8), sh, (c.size, c.size)) > 0
                a, b = flat(img)[core].ravel(), moved[core].ravel()
                ncc = float(np.corrcoef(a, b)[0, 1]) if a.std() > 0 and b.std() > 0 else 1.0
                if ncc < worst["R3"][0]:
                    worst["R3"] = (ncc, f"{label}/{f.name}")
            sheet.append((max(dd.values()), label, f, s))
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
    return res, dmax, sheet, defs


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
        make_sheet(sheet, out / name / "sheet_motion.png", a.size)
        rows = fres + rres
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
