"""마스크로 레이어를 잘라 characters/<이름>/layers/ 에 저장하고 manifest.toml 을 만든다 (PLAN 3.5-5).

규칙은 단순하게 "마스크 안은 변형 그림, 밖은 base" 다. 앱은 마스크가 있는 레이어를
"마스크 안을 교체" 로 그리므로(base 의 귀도 지울 수 있음) 레이어에는 변형 그림을, 마스크는 따로 저장한다.
경계는 페더링(가우시안)으로 섞는다.

앞발과 제스처는 base 와 실제로 달라진 곳(앞발)만 변형 그림을 쓰고 나머지(책상, 키보드)는 base 를 그대로 둔다.
생성 그림은 책상 선이 몇 px 씩 어긋나 있어서, 그대로 쓰면 앞발이 바뀔 때마다 책상이 흔들려 보인다.

마우스 자세는 슥슥 움직이는 연출을 위해 두 장으로 나눈다.
- fx/mouse_arm: 팔/손/마우스만 잘라낸 그림. 앱이 어깨(root)에서 손끝(tip)으로 갈수록 크게 휘게 그린다.
- paw_r/mouse_bg: 팔을 치운 바탕. 팔이 움직이면 드러나는 마우스패드 자리는 base 에서 마우스를 지워 채운다.

캐릭터 고유 행동 (docs/CHARACTER_BEHAVIOR_PLAN.md):
- mask/body: 머리 + 앞발 마스크를 넓힌 것. 몸 전체 움직임(hop, tremble 등)을 이 안으로 한정한다 (책상은 제자리).
- base_clean: picks.json 에 "base_clean" 이 있으면 캐릭터를 지운 책상 그림. 몸이 움직여 드러나는 자리를 채운다.
- fx/white: behavior 가 쓰면 base 실루엣 모양 흰색 (flash 소품).
- picks.json 의 "_behaviors" -> [[behavior]], "_scenes" -> [[scene]], "_timing"/"_anim" -> [timing]/[anim].

표정/책상 소품:
- "_expressions": {"head/<이름>": "<애니 근거>"} -> [expressions]. 여기 없는 머리는 앱이 그리지 않는다 (head/normal 은 항상).
- "_desk": {"item": "<소품 id>"} -> [desk]. 소품 그림은 characters/_props (make_props.py) 를 모든 캐릭터가 같이 쓴다.
  장면 프레임(full/*)이 소품 자리를 덮으면 [desk.hide] 에 그 자리를 적어 그 프레임 동안 소품을 숨긴다.
- "desk_empty": "<후보>" 가 있으면 (예전에 책상 소품까지 그려 넣은 캐릭터) 모든 레이어에서 소품 자리(art/_desk/item_mask.png)를
  빈 책상으로 바꾼다. 장면 프레임은 캐릭터가 덮은 곳(base 와 달라진 곳)은 그대로 둔다.
- "_erase": [[x0, y0, x1, y1], ..]: base 에 잘못 그려진 자국(예: 입 아래 떠 있는 선)을 상자 안 피부색으로 지운다.
  변형 그림도 그 상자 안에서 base 와 같은 곳만 같이 지운다 (표정이 바뀐 곳은 그대로).

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


# 표 이름 -> 하위 표 (목록이면 [[x.k]], 하나면 [x.k])
SUB_LISTS = ("step", "fx", "motion")
SUB_TABLES = ("breathe", "head")


def entry_layers(e: dict) -> list[str]:
    head = e.get("head", {})
    return (list(e.get("frames", [])) + [st["frame"] for st in e.get("step", [])] + [f["layer"] for f in e.get("fx", [])]
            + ([head["layer"]] if "layer" in head else []) + list(head.get("frames", [])))


def table_toml(kind: str, name: str, e: dict, names: list[str], full: bool) -> str | None:
    """[[scene]] 이나 [[behavior]] 하나. full 인데 frames/step 이 없으면 full/<name>_a, _b .. (만들어진 레이어만)."""
    lines = [f'name = "{name}"'] + [f"{k} = {toml_val(e[k])}" for k in ("trigger", "on") if k in e]
    if full and "frames" not in e and "step" not in e:
        frames = sorted(n for n in names if n.startswith(f"full/{name}_"))
        if not frames:
            print(f"{kind} {name}: 프레임 없음, 건너뜀")
            return None
        lines.append("frames = " + toml_val(frames))
    missing = [x for x in entry_layers(e) if x not in names]
    if missing:
        print(f"{kind} {name}: 없는 레이어 {missing}, 건너뜀")
        return None
    for k, v in e.items():
        if k not in SUB_LISTS + SUB_TABLES + ("trigger", "on"):
            lines.append(f"{k} = {toml_val(v)}")
    body = f"\n[[{kind}]]\n" + "\n".join(lines) + "\n"
    for k in SUB_TABLES:
        if k in e:
            body += f"\n[{kind}.{k}]\n" + "".join(f"{a} = {toml_val(b)}\n" for a, b in e[k].items())
    for k in SUB_LISTS:
        for item in e.get(k, []):
            body += f"\n[[{kind}.{k}]]\n" + "".join(f"{a} = {toml_val(b)}\n" for a, b in item.items())
    return body


def scenes_toml(picks: dict, names: list[str]) -> str:
    """picks.json 의 "_scenes" -> [[scene]] 표 (예전 형식, 그대로 읽힌다).

    예: {"eat": {"trigger": "idle", "frame_ms": 450}} 이면 프레임은 full/eat_a, full/eat_b .. 순서 (만들어진 레이어만).
    "step": [{"frame", "ms", "blend_ms"}], "breathe": {...}, "fx": [{...}] 가 있으면 [[scene.step]] [scene.breathe] [[scene.fx]] 로 쓴다.
    """
    out = [b for name, sc in picks.get("_scenes", {}).items() if (b := table_toml("scene", name, sc, names, True))]
    if not out:
        return ""
    return "\n# 애니 장면: trigger(gesture=과열이 식을 때, idle=한동안 입력 없음, milestone=키 N번째). step/breathe/fx 는 characters/README.md\n" + "".join(out)


def behaviors_toml(picks: dict, names: list[str]) -> str:
    """picks.json 의 "_behaviors" -> [[behavior]] 표.

    예: {"ura": {"on": "level:excited", "duration_ms": 900, "fx": [...], "motion": [...]}}.
    layer_mode 가 "full" 이거나 frames/step 이 있으면 전체 프레임. full 인데 frames 가 없으면 full/<이름>_a, _b ..
    "head": {"layer" | "frames", "frame_ms"} 는 [behavior.head] 로 쓴다.
    """
    out = []
    for name, b in picks.get("_behaviors", {}).items():
        full = b.get("layer_mode") == "full" or "frames" in b or "step" in b
        if (t := table_toml("behavior", name, b, names, full)):
            out.append(t)
    if not out:
        return ""
    return "\n# 캐릭터 고유 행동: on(cue), 표현(layer_mode, motion, fx, head). characters/README.md, art/<이름>/BEHAVIORS.md\n" + "".join(out)


def section_toml(picks: dict, key: str, table: str) -> str:
    """picks.json 의 "_timing"/"_anim" -> [timing]/[anim] (성격 타이밍)."""
    d = picks.get(key)
    if not d:
        return ""
    return f"\n[{table}]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in d.items())


def expressions_toml(picks: dict, names: list[str]) -> str:
    """picks.json 의 "_expressions" -> [expressions]. 레이어가 없는 표정은 뺀다."""
    ex = {k: v for k, v in picks.get("_expressions", {}).items() if k in names}
    for k in picks.get("_expressions", {}):
        if k not in names:
            print(f"표정 {k}: 레이어 없음, 뺌")
    if not ex:
        return "\n# 애니에 나온 표정만 그린다 (head/normal 만 허용)\n[expressions]\n"
    return "\n# 애니에 나온 표정만 그린다: 머리 레이어 = 근거. 여기 없는 머리는 그리지 않는다\n[expressions]\n" + "".join(
        f"{toml_val(k)} = {toml_val(v)}\n" for k, v in ex.items())


def desk_slots() -> dict:
    """소품 자리별 상자 (캔버스 px): foot 기준 max 크기."""
    spec = json.loads((ROOT / "art/_props/props.json").read_text())
    out = {}
    for name, sl in spec["slots"].items():
        (fx, fy), (w, h) = sl["foot"], sl["max"]
        out[name] = (int(fx - w / 2), int(fy - h), int(fx + w / 2), int(fy))
    return out


def desk_toml(picks: dict, hide: dict) -> str:
    d = picks.get("_desk")
    if not d and not hide:
        return ""
    body = "\n# 책상 소품: 캐릭터 소품(item)과 장면 프레임이 덮는 자리(hide). characters/_props/props.toml\n[desk]\n"
    if d and d.get("item"):
        body += f"item = {toml_val(d['item'])}\n"
    if hide:
        body += "\n[desk.hide]\n" + "".join(f"{toml_val(k)} = {toml_val(v)}\n" for k, v in sorted(hide.items()))
    return body


def erase_marks(boxes: list) -> None:
    """base 의 자국을 지우고, 변형 그림은 상자 안에서 원래 base 와 같은 곳만 같이 지운다."""
    bp = WORK / "base.png"
    base = np.array(Image.open(bp).convert("RGBA"))
    orig = base.copy()
    for x0, y0, x1, y1 in boxes:
        reg = base[y0:y1, x0:x1, :3].astype(int)
        lum = reg.sum(-1)
        skin = np.median(reg[lum >= np.percentile(lum, 60)], axis=0)
        off = np.abs(reg - skin).sum(-1) > 6
        reg[off] = skin
        base[y0:y1, x0:x1, :3] = reg
    Image.fromarray(base, "RGBA").save(bp)
    for vp in sorted((WORK / "matched").glob("*.png")):
        v = np.array(Image.open(vp).convert("RGBA"))
        changed_any = False
        for x0, y0, x1, y1 in boxes:
            same = np.abs(v[y0:y1, x0:x1, :3].astype(int) - orig[y0:y1, x0:x1, :3].astype(int)).sum(-1) < 60
            if same.any():
                v[y0:y1, x0:x1, :3][same] = base[y0:y1, x0:x1, :3][same]
                changed_any = True
        if changed_any:
            Image.fromarray(v, "RGBA").save(vp)
    print(f"자국 지움: {boxes}")


def declutter(out: Path, names: list[str], empty: np.ndarray, chg: dict) -> dict:
    """예전 책상 소품을 모든 레이어에서 지우고, 장면 프레임마다 캐릭터가 덮는 소품 자리를 돌려준다."""
    m = np.array(Image.open(ROOT / "art/_desk/item_mask.png").convert("L")).astype(np.float32) / 255
    m = cv2.GaussianBlur(m, (9, 9), 3)
    slots = desk_slots()
    hide = {}
    for n in names:
        if n.startswith(("mask/", "fx/")):
            continue
        p = out / "layers" / f"{n}.png"
        v = np.array(Image.open(p).convert("RGBA")).astype(np.float32)
        w = m.copy()
        if n in chg:
            # 캐릭터가 덮은 곳(달라진 곳)은 그대로. 넓혀서 팔 가장자리도 남긴다
            c = cv2.dilate((chg[n] > 0.05).astype(np.uint8), np.ones((9, 9), np.uint8)).astype(np.float32)
            w *= 1 - c
            if n.startswith("full/"):
                covered = [s for s, (x0, y0, x1, y1) in slots.items() if c[y0:y1, x0:x1].sum() > 150]
                if covered:
                    hide[n] = covered
        w *= v[..., 3] > 0
        # premultiplied 로 섞는다 (책상 위로 솟은 소품 자리는 빈 책상에선 투명이라 알파도 같이 바뀐다)
        v = blend(v.astype(np.uint8), empty.astype(np.uint8), w)
        Image.fromarray(v, "RGBA").save(p, optimize=False, compress_level=3)
    print(f"예전 책상 소품 지움, 소품을 숨길 장면 프레임: {hide}")
    return hide


def uses_layer(picks: dict, layer: str) -> bool:
    return any(layer in entry_layers(e) for sec in ("_scenes", "_behaviors") for e in picks.get(sec, {}).values())


def keyboard_ramp(shape, size: int, tips: dict) -> np.ndarray:
    """키보드 윗줄(앞발 끝 좌표) 위는 1, 아래는 0 인 세로 램프. 몸이 움직여도 그 아래(책상/키보드)는 제자리."""
    tip_y = [v[1] for k, v in tips.items() if k in ("paw_l/down", "paw_r/down", "paw_r/center")]
    if not tip_y:
        return np.ones(shape, np.float32)
    cut, fade = min(tip_y) - size * 0.02, size * 0.02
    yy = np.arange(shape[0], dtype=np.float32)[:, None]
    return np.broadcast_to(np.clip((cut - yy) / fade, 0, 1), shape).astype(np.float32)


def body_mask(masks: dict, size: int, tips: dict) -> tuple[np.ndarray, list[float]]:
    """몸 마스크와 몸 기준점 (머리 가운데 x, 몸 아래 끝 y).

    머리와 앞발 사이 몸통을 메우고, 키보드 윗줄 아래는 서서히 뺀다 (책상/키보드는 제자리).
    앞발 마스크는 같은 선에서 위(paw_*_top, 몸과 같이 움직임)와 아래(paw_*_low, 제자리)로 나눈다.
    """
    paws = np.maximum(masks["paw_l"], masks["paw_r"])
    ramp = keyboard_ramp(paws.shape, size, tips)
    # 구멍 없는 한 덩어리: 키보드 윗줄 위의 머리 + 팔을 볼록 껍질로 감싸고, 몸 폭(머리 폭 + 여유)으로 자른다.
    # 구멍이 있으면 가장자리 페더링 때문에 제자리 그림과 옮긴 그림이 반씩 비쳐 이음선이 생긴다
    u = ((np.maximum(masks["head"], paws) > 0.5) & (ramp > 0)).astype(np.uint8)
    m = np.zeros_like(u)
    cv2.fillConvexPoly(m, cv2.convexHull(cv2.findNonZero(u)), 1)
    # 몸통은 머리/팔 마스크보다 옆으로 불룩하다 (치이카와 아래쪽 등). 넉넉히 넓힌 뒤 소품(연필꽂이, 병)이 안 들어가게 폭을 자른다
    g = max(5, size // 25)
    m = cv2.dilate(m, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (g * 2 + 1, g * 2 + 1)))
    _, hx = np.nonzero(masks["head"] > 0.5)
    pad = size // 20
    m[:, : max(0, hx.min() - pad)] = 0
    m[:, hx.max() + pad :] = 0
    r = max(3, size // 100)
    m = cv2.GaussianBlur(m.astype(np.float32), (r * 2 + 1, r * 2 + 1), r / 2) * ramp
    ys, _ = np.nonzero(m > 0.5)
    return m, [round(float(hx.mean()), 1), round(float(ys.max()), 1)]


def split_paw_masks(masks: dict, size: int, tips: dict) -> dict:
    ramp = keyboard_ramp(masks["paw_l"].shape, size, tips)
    out = {}
    for g in ("paw_l", "paw_r"):
        out[f"{g}_top"] = masks[g] * ramp
        out[f"{g}_low"] = masks[g] * (1 - ramp)
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", default="usagi")
    ap.add_argument("--feather", type=int, default=6)
    a = ap.parse_args()
    out = ROOT / "characters" / a.name
    set_char(a.name)
    picks = json.loads((ROOT / "art" / a.name / "picks.json").read_text())
    geo = json.loads((WORK / "geometry.json").read_text())
    if picks.get("_erase"):
        erase_marks(picks["_erase"])

    masks = {g: feathered(g, a.feather) for g in ["head", "paw_l", "paw_r"]}
    NOT_LAYERS = ("base", "desk_empty")
    srcs = {k: WORK / "matched" / f"{v}.png" for k, v in picks.items() if not k.startswith("_") and k not in NOT_LAYERS}
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
    masks["body"], body_anchor = body_mask(masks, masks["head"].shape[0], geo["tips"])
    masks.update(split_paw_masks(masks, masks["head"].shape[0], geo["tips"]))
    for g, m in masks.items():
        save_layer(out, f"mask/{g}", np.dstack([(m * 255).astype(np.uint8)] * 3 + [np.full(m.shape, 255, np.uint8)]))

    jobs = [(out, "base", WORK / "base.png", None, None), (out, "head/normal", WORK / "base.png", masks["head"], None),
            (out, "paw_l/up", WORK / "base.png", masks["paw_l"], None), (out, "paw_r/up", WORK / "base.png", masks["paw_r"], None)]
    for layer, raw in picks.items():
        if layer.startswith("_") or layer in NOT_LAYERS:
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
    # 장면 소품 (art/scripts/extract_fx.py 가 work/fx/ 에 잘라 둔 것): 그대로 fx/ 로
    for f in sorted((WORK / "fx").glob("*.png")) if (WORK / "fx").is_dir() else []:
        save_layer(out, f"fx/{f.stem}", np.array(Image.open(f).convert("RGBA")))
        names.append(f"fx/{f.stem}")
    # 플래시 소품: base 실루엣 모양 흰색 (behavior 가 쓸 때만)
    if uses_layer(picks, "fx/white"):
        alpha = np.array(Image.open(WORK / "base.png").convert("RGBA"))[..., 3]
        save_layer(out, "fx/white", np.dstack([np.full(alpha.shape, 255, np.uint8)] * 3 + [alpha]))
        names.append("fx/white")
    hide = {}
    if "desk_empty" in picks:
        empty = np.array(Image.open(WORK / "matched" / f"{picks['desk_empty']}.png").convert("RGBA")).astype(np.float32)
        layer_names = [n for n in names if (out / "layers" / f"{n}.png").exists()]
        hide = declutter(out, layer_names, empty, chg)
    names += ["mask/head", "mask/paw_l", "mask/paw_r", "mask/body", "mask/paw_l_top", "mask/paw_l_low", "mask/paw_r_top", "mask/paw_r_low"]

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
body_anchor = [{body_anchor[0]}, {body_anchor[1]}]

[masks]
head = "mask/head"
paw_l = "mask/paw_l"
paw_r = "mask/paw_r"
body = "mask/body"
paw_l_top = "mask/paw_l_top"
paw_l_low = "mask/paw_l_low"
paw_r_top = "mask/paw_r_top"
paw_r_low = "mask/paw_r_low"

[tips]
{tips}
{swing_toml(swing)}{section_toml(picks, "_timing", "timing")}{section_toml(picks, "_anim", "anim")}{expressions_toml(picks, names)}{desk_toml(picks, hide)}{scenes_toml(picks, names)}{behaviors_toml(picks, names)}""",
        encoding="utf-8",
    )
    print(f"{out.relative_to(ROOT)}: 레이어 {len(names)}장")


if __name__ == "__main__":
    main()
