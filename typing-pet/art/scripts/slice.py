"""마스크로 레이어를 잘라 characters/<이름>/layers/ 에 저장하고 manifest.toml 을 만든다 (PLAN 3.5-5).

규칙은 단순하게 "마스크 안은 변형 그림, 밖은 base" 다. 앱은 마스크가 있는 레이어를
"마스크 안을 교체" 로 그리므로(base 의 귀도 지울 수 있음) 레이어에는 변형 그림을, 마스크는 따로 저장한다.
경계는 페더링(가우시안)으로 섞는다.

앞발과 제스처는 base 와 실제로 달라진 곳(앞발)만 변형 그림을 쓰고 나머지(책상, 키보드)는 base 를 그대로 둔다.
생성 그림은 책상 선이 몇 px 씩 어긋나 있어서, 그대로 쓰면 앞발이 바뀔 때마다 책상이 흔들려 보인다.

마우스 자세는 슥슥 움직이는 연출을 위해 두 장으로 나눈다.
- fx/mouse_arm: 팔/손/마우스만 잘라낸 그림. 앱이 어깨(root)에서 손끝(tip)으로 갈수록 크게 휘게 그린다.
- paw_r/mouse_bg: 팔을 치운 바탕. 팔이 움직이면 드러나는 마우스패드 자리는 base 에서 마우스를 지워 채운다.

사용법: slice.py [--name usagi] [--feather 6]

캐릭터별 폴더 art/<이름>/ 의 work/, masks/, picks.json 을 읽는다.
"""

import argparse
import json
import os
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "art/usagi/work"
MASKS = ROOT / "art/usagi/masks"
if os.environ.get("TYPING_PET_ART_CHAR"):
    WORK = ROOT / "art" / os.environ["TYPING_PET_ART_CHAR"] / "work"
    MASKS = ROOT / "art" / os.environ["TYPING_PET_ART_CHAR"] / "masks"
GROUP_OF = {"head": "head", "paw_l": "paw_l", "paw_r": "paw_r"}


def set_char(name: str) -> None:
    """작업 폴더를 art/<name>/ 로. 프로세스 풀 작업자도 같은 값을 쓰도록 환경 변수로도 남긴다."""
    global WORK, MASKS
    os.environ["TYPING_PET_ART_CHAR"] = name
    WORK, MASKS = ROOT / "art" / name / "work", ROOT / "art" / name / "masks"


def feathered(name: str, feather: int) -> np.ndarray:
    m = np.array(Image.open(MASKS / f"{name}.png").convert("L")).astype(np.float32) / 255
    k = feather * 2 + 1
    return cv2.GaussianBlur(m, (k, k), feather / 2)


def save_layer(out: Path, name: str, rgba: np.ndarray) -> None:
    p = out / "layers" / f"{name}.png"
    p.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgba, "RGBA").save(p, optimize=False, compress_level=3)


def flat(rgba: np.ndarray) -> np.ndarray:
    a = rgba[..., 3:4].astype(np.float32) / 255
    return rgba[..., :3].astype(np.float32) * a + 255 * (1 - a)


def shift_diff(a: np.ndarray, b: np.ndarray, r: int) -> np.ndarray:
    """a 의 각 픽셀과, b 에서 r px 안의 가장 비슷한 픽셀의 색 차이."""
    h, w = a.shape[:2]
    pb = np.pad(b, ((r, r), (r, r), (0, 0)), mode="edge")
    best = np.full((h, w), np.inf, np.float32)
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            d = np.abs(a - pb[r + dy:r + dy + h, r + dx:r + dx + w]).max(axis=-1)
            np.minimum(best, d, out=best)
    return best


def changed(base: np.ndarray, var: np.ndarray, thr: float = 48, shift: int = 4, feather: int = 6) -> np.ndarray:
    """base 와 실제로 다른 영역 (0~1). 몇 px 어긋나기만 한 선은 바뀐 것으로 치지 않는다.

    양쪽 방향으로 본다: 변형에 새로 생긴 것(var 픽셀이 base 근처 어디에도 없음)과
    사라진 것(base 픽셀이 var 근처 어디에도 없음, 예: 올렸던 앞발의 외곽선).
    """
    fb, fv = flat(base), flat(var)
    d = np.maximum(shift_diff(fv, fb, shift), shift_diff(fb, fv, shift))
    m = (cv2.GaussianBlur(d, (3, 3), 0) > thr).astype(np.uint8)
    n, lab, stats, _ = cv2.connectedComponentsWithStats(m, 8)
    keep = np.zeros_like(m)
    for i in range(1, n):
        if stats[i, cv2.CC_STAT_AREA] >= 40:
            keep[lab == i] = 1
    keep = cv2.morphologyEx(keep, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (25, 25)))
    keep = cv2.dilate(keep, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (19, 19)))
    k = feather * 2 + 1
    # 바뀐 픽셀은 정확히 1 이 되게 (조금이라도 base 가 섞이면 지운 외곽선이 희미하게 남는다)
    return np.clip(cv2.GaussianBlur(keep.astype(np.float32), (k, k), feather / 2) * 1.25, 0, 1)


def blend(base: np.ndarray, var: np.ndarray, w: np.ndarray) -> np.ndarray:
    """premultiplied 로 섞어서 RGBA uint8 로 돌려준다."""
    def pre(x):
        x = x.astype(np.float32)
        return np.dstack([x[..., :3] * x[..., 3:4] / 255, x[..., 3]])
    w = w[..., None]
    p = pre(base) * (1 - w) + pre(var) * w
    a = p[..., 3:4]
    rgb = np.where(a > 1e-3, p[..., :3] * 255 / np.maximum(a, 1e-3), 0)
    return np.dstack([np.clip(rgb, 0, 255), np.clip(a, 0, 255)]).astype(np.uint8)


def load_var(src: Path) -> np.ndarray:
    """생성 그림은 크로마키 뒤 거의 불투명(250~254)한 픽셀이 지운 선 모양으로 남아 있어서 255 로 맞춘다."""
    v = np.array(Image.open(src).convert("RGBA"))
    v[..., 3][v[..., 3] >= 250] = 255
    return v


def change_job(src: Path) -> np.ndarray:
    base = np.array(Image.open(WORK / "base.png").convert("RGBA"))
    return changed(base, load_var(src))


def job(args):
    out, name, src, mask, chg = args
    rgba = load_var(src)
    if chg is not None:
        base = np.array(Image.open(WORK / "base.png").convert("RGBA"))
        rgba = blend(base, rgba, chg)
    if mask is not None:
        # 마스크 밖은 쓰이지 않으므로 비워서 파일을 작게
        rgba[mask <= 0.0] = 0
    save_layer(out, name, rgba)
    return name


def base_mouse(base: np.ndarray, near: list[float]) -> np.ndarray | None:
    """base 에서 near(마우스를 누른 손끝) 근처의 흰 덩어리 = 마우스. 몸, 키캡처럼 크거나 먼 덩어리는 뺀다."""
    bw = ((base[..., :3].min(axis=-1) > 215) & (base[..., 3] > 128)).astype(np.uint8)
    n, lab, st, c = cv2.connectedComponentsWithStats(bw, 8)
    cands = [j for j in range(1, n) if 300 < st[j, cv2.CC_STAT_AREA] < 8000 and np.hypot(*(c[j] - near)) < 90]
    if not cands:
        return None
    return fill_holes(lab == min(cands, key=lambda j: np.hypot(*(c[j] - near))))


def body_is_white(base: np.ndarray) -> bool:
    """base 에서 가장 큰 흰 덩어리가 캔버스의 5% 넘게 차지하면 몸이 흰색이다."""
    bw = ((base[..., :3].min(axis=-1) > 215) & (base[..., 3] > 128)).astype(np.uint8)
    n, _, st, _ = cv2.connectedComponentsWithStats(bw, 8)
    return n > 1 and st[1:, cv2.CC_STAT_AREA].max() > bw.size * 0.05


def fill_holes(m: np.ndarray) -> np.ndarray:
    m = m.astype(np.uint8) * 255
    h, w = m.shape
    ff = m.copy()
    cv2.floodFill(ff, np.zeros((h + 2, w + 2), np.uint8), (0, 0), 255)
    return (m | cv2.bitwise_not(ff)) > 0


def cream_arm(base, v, mask, chg, rgb, alpha, xx, yy):
    """몸이 크림색(우사기): 몸 색 픽셀이 팔, 흰 덩어리가 마우스. (sprite, root, tip, near)"""
    h, w = mask.shape
    inside = (chg > 0.5) & (mask > 0.5)
    body = base[h // 2 - 100, w // 2, :3].astype(np.int32)  # 몸 가운데 크림색
    cream = (np.abs(rgb - body).max(axis=-1) < 28) & (alpha > 128)
    white = (rgb.min(axis=-1) > 228) & (alpha > 128)
    dark = (rgb.max(axis=-1) < 120) & (alpha > 128)
    # 손끝: 앞발 영역 안 흰 덩어리(마우스) 중 가장 큰 것의 중심
    n, lab, stats, cents = cv2.connectedComponentsWithStats((white & inside).astype(np.uint8), 8)
    i = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
    tip = cents[i]
    near = (xx - tip[0]) ** 2 + (yy - tip[1]) ** 2 < 80 ** 2
    # 마우스(흰색)는 바뀐 영역 가장자리까지 다 넣어야 바탕에 잔상이 안 남는다
    loose = (chg > 0.05) | (mask > 0.05)
    core = ((cream & inside) | (white & near & loose)).astype(np.uint8)
    core = cv2.morphologyEx(core, cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
    n, lab, stats, _ = cv2.connectedComponentsWithStats(core, 8)
    keep = np.isin(lab, [j for j in range(1, n) if stats[j, cv2.CC_STAT_AREA] > 300])
    adj = cv2.dilate(keep.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (27, 27))) > 0
    sprite = keep | (dark & adj & (cv2.dilate(inside.astype(np.uint8), np.ones((31, 31), np.uint8)) > 0))
    sprite = cv2.morphologyEx(sprite.astype(np.uint8), cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8)) > 0
    # 어깨: 손끝 위쪽 150px 보다 높은 팔 픽셀의 중심
    band = sprite & (yy < tip[1] - 150) & (np.abs(xx - tip[0]) < 110)
    ys, xs = np.nonzero(band)
    root = [float(xs.mean()), float(ys.mean())] if len(xs) else [float(tip[0]) - 60, float(tip[1]) - 170]
    return sprite, root, tip, near


def white_arm(base, v, mask, chg):
    """몸이 흰색: 마우스 자세에서 base 의 몸 윤곽 밖으로 나온 흰 덩어리 중 가장 큰 것이 팔.
    root 는 팔이 몸에 붙는 곳, tip 은 팔 끝 근처 마우스의 중심(없으면 팔 끝). (sprite, root, tip)"""
    h, w = mask.shape
    yy, xx = np.mgrid[:h, :w]
    rgb, alpha = v[..., :3].astype(np.int32), v[..., 3]
    white = (rgb.min(axis=-1) > 215) & (alpha > 128)
    dark = (rgb.max(axis=-1) < 120) & (alpha > 128)
    bwhite = ((base[..., :3].min(axis=-1) > 215) & (base[..., 3] > 128)).astype(np.uint8)
    n, lab, st, _ = cv2.connectedComponentsWithStats(bwhite, 8)
    body = fill_holes(lab == 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA])))
    body_d = cv2.dilate(body.astype(np.uint8), np.ones((7, 7), np.uint8)) > 0
    inside = (chg > 0.3) & (mask > 0.3)
    cand = cv2.morphologyEx((white & inside & ~body_d).astype(np.uint8), cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
    n, lab, st, _ = cv2.connectedComponentsWithStats(cand, 8)
    arm = lab == 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA]))
    # 팔 끝은 몸에서 가장 먼 5%, 어깨는 가장 가까운 5%
    dist = cv2.distanceTransform((~body).astype(np.uint8), cv2.DIST_L2, 5)
    da = dist[arm]
    ys, xs = np.nonzero(arm)
    far, close = da >= np.percentile(da, 95), da <= np.percentile(da, 5)
    hand = np.array([xs[far].mean(), ys[far].mean()])
    root = [float(xs[close].mean()), float(ys[close].mean())]
    # 마우스: base 에서 손 근처의 흰 덩어리 (몸, 키캡 제외). 바뀐 영역 가장자리까지 넣어야 바탕에 잔상이 안 남는다
    bm = (bwhite > 0) & ~body_d
    n2, lab2, st2, c2 = cv2.connectedComponentsWithStats(bm.astype(np.uint8), 8)
    cands = [j for j in range(1, n2) if 300 < st2[j, cv2.CC_STAT_AREA] < 8000 and np.hypot(*(c2[j] - hand)) < 90]
    tip = c2[min(cands, key=lambda j: np.hypot(*(c2[j] - hand)))] if cands else hand
    near = (xx - tip[0]) ** 2 + (yy - tip[1]) ** 2 < 60 ** 2
    loose = (chg > 0.05) | (mask > 0.05)
    keep = arm | (white & near & loose & ~body_d)
    keep = cv2.morphologyEx(keep.astype(np.uint8), cv2.MORPH_OPEN, np.ones((3, 3), np.uint8)) > 0
    adj = cv2.dilate(keep.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (17, 17))) > 0
    sprite = keep | (dark & adj & ~body_d)
    sprite = cv2.morphologyEx(sprite.astype(np.uint8), cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8)) > 0
    return sprite, root, tip


def mouse_swing(out: Path, src: Path, mask: np.ndarray, chg: np.ndarray) -> dict:
    """마우스 자세를 팔(fx/mouse_arm)과 바탕(paw_r/mouse_bg)으로 나누고 휘는 축(root -> tip)을 돌려준다."""
    base = np.array(Image.open(WORK / "base.png").convert("RGBA"))
    v = blend(base, load_var(src), chg)
    h, w = mask.shape
    yy, xx = np.mgrid[:h, :w]
    rgb, alpha = v[..., :3].astype(np.int32), v[..., 3]
    body_white = body_is_white(base)
    if body_white:
        # 몸이 흰색(치이카와, 하치와레)이면 몸과 마우스를 색으로 못 가른다. 몸 윤곽 밖으로 나온 흰 부분을 팔로 본다
        sprite, root, tip = white_arm(base, v, mask, chg)
        near = (xx - tip[0]) ** 2 + (yy - tip[1]) ** 2 < 80 ** 2
    else:
        sprite, root, tip, near = cream_arm(base, v, mask, chg, rgb, alpha, xx, yy)

    # 마우스 외곽선도 팔과 같이 움직인다
    mw = (rgb.min(axis=-1) > 215) & (alpha > 128) & near & (mask > 0.05)
    mdark = (rgb.max(axis=-1) < 120) & (alpha > 128)
    sprite |= mdark & (cv2.dilate(mw.astype(np.uint8), np.ones((9, 9), np.uint8)) > 0) & (mask > 0.05)
    arm = v.copy()
    sa = cv2.GaussianBlur(sprite.astype(np.float32), (3, 3), 0) * np.clip(mask * 1.5, 0, 1)
    arm[..., 3] = (arm[..., 3].astype(np.float32) * sa).astype(np.uint8)
    save_layer(out, "fx/mouse_arm", arm)

    # 바탕: 팔 자리는 base 로, base 의 마우스는 패드 색으로 지운다
    bw = (base[..., :3].min(axis=-1) > 228) & (base[..., 3] > 128) & near
    # 마우스 바로 둘레에서 패드 색을 고른다. 흰 몸 캐릭터는 패드 여백이 좁아 넓게 보면 책상 색이 뽑힌다
    r_out, r_in = (19, 9) if body_white else (31, 15)
    ring = (cv2.dilate(bw.astype(np.uint8), np.ones((r_out, r_out), np.uint8)) > 0) & ~(cv2.dilate(bw.astype(np.uint8), np.ones((r_in, r_in), np.uint8)) > 0)
    cand = base[ring & (base[..., :3].max(axis=-1) > 150)][:, :3].astype(np.int32) // 8
    keys, cnt = np.unique(cand[:, 0] * 10000 + cand[:, 1] * 100 + cand[:, 2], return_counts=True)
    k = int(keys[cnt.argmax()]) if len(keys) else 0
    pad = np.array([k // 10000, (k // 100) % 100, k % 100]) * 8 + 4
    fixed = base.copy()
    erase = cv2.dilate(bw.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (29, 29))) > 0
    fixed[erase, :3] = pad
    fixed[erase, 3] = 255
    # 손끝 근처(마우스패드)는 base 에서, 팔 가운데는 주변 색으로 메운다 (base 에는 올린 앞발이 있어서 못 쓴다)
    hole_m = (cv2.dilate(sprite.astype(np.uint8), np.ones((7, 7), np.uint8)) > 0).astype(np.uint8)
    filled = v.copy()
    filled[..., :3] = cv2.inpaint(np.ascontiguousarray(v[..., :3]), hole_m, 9, cv2.INPAINT_TELEA)
    filled[..., 3] = cv2.inpaint(np.ascontiguousarray(v[..., 3]), hole_m, 9, cv2.INPAINT_TELEA)
    dist = np.sqrt((xx - tip[0]) ** 2 + (yy - tip[1]) ** 2)
    pad_w = 1.0 - np.clip((dist - 75) / 30, 0, 1)
    filled = blend(filled, fixed, pad_w.astype(np.float32))
    hole = cv2.GaussianBlur(hole_m.astype(np.float32), (9, 9), 3)
    bg = blend(v, filled, hole)
    bg[mask <= 0.0] = 0
    save_layer(out, "paw_r/mouse_bg", bg)
    print(f"마우스 팔: root={[round(c, 1) for c in root]} tip={[round(float(c), 1) for c in tip]} 팔 {int(sprite.sum())}px")
    return {"root": [round(c, 1) for c in root], "tip": [round(float(c), 1) for c in tip]}


def swing_toml(sw: dict | None) -> str:
    if not sw:
        return ""
    return f"""
# 마우스 연출: fx/mouse_arm 을 root(어깨) 에서 tip(손끝) 으로 갈수록 크게 휘게 그린다 (캔버스 px)
[swing]
root = [{sw['root'][0]}, {sw['root'][1]}]
tip = [{sw['tip'][0]}, {sw['tip'][1]}]
"""


def toml_val(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return repr(v)
    if isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, list):
        return "[" + ", ".join(toml_val(x) for x in v) + "]"
    raise TypeError(v)


def scenes_toml(picks: dict, names: list[str]) -> str:
    """picks.json 의 "_scenes" -> [[scene]] 표.

    예: {"eat": {"trigger": "idle", "frame_ms": 450}} 이면 프레임은 full/eat_a, full/eat_b .. 순서 (만들어진 레이어만).
    "step": [{"frame", "ms", "blend_ms"}], "breathe": {...}, "fx": [{...}] 가 있으면 [[scene.step]] [scene.breathe] [[scene.fx]] 로 쓴다.
    """
    out = []
    for name, sc in picks.get("_scenes", {}).items():
        lines = [f'name = "{name}"', f'trigger = "{sc["trigger"]}"']
        if "step" not in sc:
            frames = sorted(n for n in names if n.startswith(f"full/{name}_"))
            if not frames:
                print(f"장면 {name}: 프레임 없음, 건너뜀")
                continue
            lines.append("frames = " + toml_val(frames))
        missing = [x for x in [st["frame"] for st in sc.get("step", [])] + [f["layer"] for f in sc.get("fx", [])] if x not in names]
        if missing:
            print(f"장면 {name}: 없는 레이어 {missing}, 건너뜀")
            continue
        for k, v in sc.items():
            if k not in ("trigger", "step", "breathe", "fx"):
                lines.append(f"{k} = {toml_val(v)}")
        body = "\n[[scene]]\n" + "\n".join(lines) + "\n"
        if "breathe" in sc:
            body += "\n[scene.breathe]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in sc["breathe"].items())
        for st in sc.get("step", []):
            body += "\n[[scene.step]]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in st.items())
        for fx in sc.get("fx", []):
            body += "\n[[scene.fx]]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in fx.items())
        out.append(body)
    if not out:
        return ""
    return "\n# 애니 장면: trigger(gesture=과열이 식을 때, idle=한동안 입력 없음, milestone=키 N번째). step/breathe/fx 는 characters/README.md\n" + "".join(out)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", default="usagi")
    ap.add_argument("--feather", type=int, default=6)
    a = ap.parse_args()
    out = ROOT / "characters" / a.name
    set_char(a.name)
    picks = json.loads((ROOT / "art" / a.name / "picks.json").read_text())
    geo = json.loads((WORK / "geometry.json").read_text())

    masks = {g: feathered(g, a.feather) for g in ["head", "paw_l", "paw_r"]}
    srcs = {k: WORK / "matched" / f"{v}.png" for k, v in picks.items() if not k.startswith("_") and k != "base"}
    srcs = {k: p for k, p in srcs.items() if p.exists()}
    # 앞발/제스처: base 와 실제로 달라진 곳만 변형을 쓴다. 앞발 마스크가 달라진 곳을 다 덮도록 넓힌다
    chg_layers = [k for k in srcs if k.split("/")[0] in ("paw_l", "paw_r", "full")]
    with ProcessPoolExecutor() as ex:
        chg = dict(zip(chg_layers, ex.map(change_job, [srcs[k] for k in chg_layers])))
    for g in ["paw_l", "paw_r"]:
        for k, c in chg.items():
            if k.startswith(g + "/"):
                masks[g] = np.maximum(masks[g], c * (1 - masks["head"]))
    # 마우스 전체를 오른발 마스크에 넣는다. 안 넣으면 마스크 밖 마우스 아래쪽이 base 에서 그대로 보여서
    # 마우스 연출 때 손만 움직이고 마우스는 제자리에 남는다
    if "paw_r/mouse" in srcs and "paw_r/mouse_click" in geo["tips"]:
        base_rgba = np.array(Image.open(WORK / "base.png").convert("RGBA"))
        mm = base_mouse(base_rgba, geo["tips"]["paw_r/mouse_click"])
        if mm is not None:
            mm = cv2.dilate(mm.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (29, 29))).astype(np.float32)
            k = a.feather * 2 + 1
            mm = np.clip(cv2.GaussianBlur(mm, (k, k), a.feather / 2) * 1.5, 0, 1)
            masks["paw_r"] = np.maximum(masks["paw_r"], mm * (1 - masks["head"]) * (1 - masks["paw_l"]))
    # 머리/앞발 마스크가 겹치는 정도 확인 (겹치면 경계에서 이전 자세가 비쳐 보인다)
    for x, y in [("head", "paw_l"), ("head", "paw_r"), ("paw_l", "paw_r")]:
        ov = np.minimum(masks[x], masks[y]).max()
        print(f"마스크 겹침 {x}/{y}: 최대 {ov:.3f}")
    for g, m in masks.items():
        save_layer(out, f"mask/{g}", np.dstack([(m * 255).astype(np.uint8)] * 3 + [np.full(m.shape, 255, np.uint8)]))

    jobs = [(out, "base", WORK / "base.png", None, None), (out, "head/normal", WORK / "base.png", masks["head"], None),
            (out, "paw_l/up", WORK / "base.png", masks["paw_l"], None), (out, "paw_r/up", WORK / "base.png", masks["paw_r"], None)]
    for layer, raw in picks.items():
        if layer.startswith("_") or layer == "base":
            continue
        if layer not in srcs:
            print(f"없음, 건너뜀: {layer} <- {raw}.png")
            continue
        group = layer.split("/")[0]
        jobs.append((out, layer, srcs[layer], masks.get(GROUP_OF.get(group, ""), None), chg.get(layer)))
    with ProcessPoolExecutor() as ex:
        names = list(ex.map(job, jobs))

    # 선글라스 단독 (fx): 선글라스 영역만, 일반 over 합성
    if (MASKS / "sunglasses.png").exists() and "head/sunglasses" in picks:
        v = np.array(Image.open(WORK / "matched" / f"{picks['head/sunglasses']}.png").convert("RGBA")).astype(np.float32)
        m = feathered("sunglasses", 1)
        v[..., 3] *= m
        save_layer(out, "fx/sunglasses", v.astype(np.uint8))
        names.append("fx/sunglasses")
    swing = None
    if "paw_r/mouse" in picks and (WORK / "matched" / f"{picks['paw_r/mouse']}.png").exists():
        swing = mouse_swing(out, srcs["paw_r/mouse"], masks["paw_r"], chg["paw_r/mouse"])
        names += ["paw_r/mouse_bg", "fx/mouse_arm"]
    # 장면 소품 (art/scripts/draw_fx.py 가 work/fx/ 에 그린 것): 그대로 fx/ 로
    for f in sorted((WORK / "fx").glob("*.png")) if (WORK / "fx").is_dir() else []:
        save_layer(out, f"fx/{f.stem}", np.array(Image.open(f).convert("RGBA")))
        names.append(f"fx/{f.stem}")
    names += ["mask/head", "mask/paw_l", "mask/paw_r"]

    order = ["base", "head/", "paw_l/", "paw_r/", "fx/", "full/", "mask/"]
    names.sort(key=lambda n: (next(i for i, p in enumerate(order) if n.startswith(p)), n))
    tips = "\n".join(f'"{k}" = [{v[0]}, {v[1]}]' for k, v in geo["tips"].items())
    layers = ",\n".join(f'  "{n}"' for n in names)
    ha = geo["head_anchor"]
    canvas = Image.open(WORK / "base.png").size[0]
    title = f'title = "{picks["_title"]}"\n' if picks.get("_title") else ""
    (out / "manifest.toml").write_text(
        f"""# art/scripts/slice.py 가 만든다. [timing] [anim] [keys] 를 넣으면 기본값을 덮어쓴다 (src/config.rs).
name = "{a.name}"
{title}canvas = {canvas}
layers = [
{layers},
]
head_anchor = [{ha[0]}, {ha[1]}]

[masks]
head = "mask/head"
paw_l = "mask/paw_l"
paw_r = "mask/paw_r"

[tips]
{tips}
{swing_toml(swing)}{scenes_toml(picks, names)}""",
        encoding="utf-8",
    )
    print(f"{out.relative_to(ROOT)}: 레이어 {len(names)}장")


if __name__ == "__main__":
    main()
