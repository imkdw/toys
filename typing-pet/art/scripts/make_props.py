"""책상 소품(직군 장비, 캐릭터 소품)을 생성 그림에서 잘라 characters/_props/ 를 만든다.

art/_props/props.json 을 읽는다.
- "sprite": art/_props/sprite/<이름>.png. 마젠타 배경에 소품 하나만 크게 그린 그림 (art/scripts/gen_props.sh).
  배경색을 빼고 자리 크기로 줄인다. 윤곽이 가장 깨끗해서 새 소품은 이 방식으로 만든다.
- "src": art/_props/raw/<이름>.png. 빈 책상(art/_desk/desk_empty.png)에 소품 하나를 그려 넣은 그림.
  빈 책상과 달라진 덩어리를 소품 자리의 search 상자 안에서 찾아 자른다.
- "from": {"src", "ref", "box", "pick"}. 기존 캐릭터 그림에서 원래 소품을 잘라 온다 (ref 는 그 소품만 없는 그림).
  pick 은 소품 몸통 안의 점들: 같이 붙어 온 책상 바탕을 벗긴 뒤 그 점이 놓인 덩어리만 남긴다 (옆 소품이 같이 잘려 오지 않게).

자른 소품은 자리(slot)의 max 크기 안으로 줄이고 바닥 가운데를 foot 에 맞춰 캔버스 크기 그대로 저장한다.
그래서 모든 캐릭터의 같은 자리에 같은 모양으로 놓인다 (책상 위치가 캐릭터마다 같다).
props.toml 에는 소품(id, 이름, 자리, foot, 누를 때 움직임)과 직군 목록을 쓴다 (src/desk.rs).

사용법: make_props.py [--thr 60] [--only coffee,duck]
"""

import argparse
import json
import tempfile
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import align
import chroma
from extract_fx import extract, prep

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "art" / "_props"
OUT = ROOT / "characters" / "_props"


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


def peel(rgba: np.ndarray, ref: np.ndarray, tol: float = 30.0) -> np.ndarray:
    """잘라 온 조각에 같이 붙어 온 책상 바탕을 벗긴다. 바탕색은 소품 없는 그림(ref)의 같은 자리 가운데값으로 잡고,
    조각 바깥과 이어진 바탕색 영역만 지운다 (소품 윤곽선 안쪽은 색이 비슷해도 남는다)."""
    a = rgba[..., 3] > 40
    if not a.any():
        return rgba
    desk = np.median(ref[a][:, :3].astype(np.float32), axis=0)
    like = a & (np.linalg.norm(rgba[..., :3].astype(np.float32) - desk, axis=2) < tol)
    n, lab = cv2.connectedComponents(like.astype(np.uint8))
    outside = cv2.dilate((~a).astype(np.uint8), np.ones((3, 3), np.uint8)).astype(bool)
    gone = np.isin(lab, np.unique(lab[outside & like]))
    out = rgba.copy()
    out[gone & like] = 0
    return out


def pick(rgba: np.ndarray, points: list[list[int]]) -> np.ndarray:
    """points 가 놓인 덩어리만 남긴다 (붙어 있는 이웃 소품을 떼어 낼 때). 점은 소품 몸통 안쪽이어야 한다."""
    n, lab = cv2.connectedComponents((rgba[..., 3] > 40).astype(np.uint8))
    keep = {int(lab[y, x]) for x, y in points} - {0}
    if not keep:
        raise SystemExit(f"pick {points}: 그 자리에 소품이 없음")
    out = rgba.copy()
    out[~np.isin(lab, list(keep))] = 0
    return out


def cut(rgba: np.ndarray, box: list[int]) -> np.ndarray:
    """box 밖은 지운다. 그 안에서 가장 큰 덩어리 묶음(가까운 조각 포함)만 남긴다."""
    x0, y0, x1, y1 = box
    out = np.zeros_like(rgba)
    out[y0:y1, x0:x1] = rgba[y0:y1, x0:x1]
    a = (out[..., 3] > 40).astype(np.uint8)
    groups = cv2.dilate(a, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (31, 31)))
    n, lab, st, _ = cv2.connectedComponentsWithStats(groups)
    if n <= 1:
        raise SystemExit("소품을 찾지 못함")
    big = 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA]))
    out[lab != big] = 0
    return out


def key(path: Path, lo: float = 70.0, hi: float = 150.0) -> np.ndarray:
    """단색 배경(마젠타)에 소품 하나만 그린 그림에서 배경을 뺀다. 배경색은 네 귀퉁이의 가운데값.
    경계는 배경색과의 거리로 부드럽게 자르고, 반투명 가장자리에 섞인 배경색을 덜어 낸다."""
    rgb = np.array(Image.open(path).convert("RGB")).astype(np.float32)
    c = 12
    bg = np.median(np.concatenate([rgb[:c, :c], rgb[:c, -c:], rgb[-c:, :c], rgb[-c:, -c:]]).reshape(-1, 3), axis=0)
    a = np.clip((np.linalg.norm(rgb - bg, axis=2) - lo) / (hi - lo), 0, 1)
    # 먼지 같은 작은 조각은 버린다
    n, lab, st, _ = cv2.connectedComponentsWithStats((a > 0.5).astype(np.uint8))
    small = [i for i in range(1, n) if st[i, cv2.CC_STAT_AREA] < 60]
    keep = cv2.dilate((~np.isin(lab, [0] + small)).astype(np.uint8), np.ones((5, 5), np.uint8)).astype(bool)
    a[~keep] = 0
    un = np.where(a[..., None] > 0.02, (rgb - (1 - a[..., None]) * bg) / np.maximum(a[..., None], 0.02), 0)
    return np.dstack([np.clip(un, 0, 255), a * 255]).astype(np.uint8)


def solid(sprite: np.ndarray) -> np.ndarray:
    """소품 안쪽은 불투명하게. 크로마키 색에 가까운 초록(브로콜리 잎)이 반투명해져 책상이 비치는 것을 막는다.
    윤곽 안을 채운 뒤 2px 안쪽부터 alpha 를 255 로 올린다 (가장자리의 부드러운 경계는 그대로)."""
    a = (sprite[..., 3] > 40).astype(np.uint8)
    cnt, _ = cv2.findContours(a, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    fill = np.zeros_like(a)
    cv2.drawContours(fill, cnt, -1, 1, cv2.FILLED)
    inner = cv2.erode(fill, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))).astype(bool)
    out = sprite.copy()
    out[..., 3][inner & (sprite[..., 3] > 0)] = 255
    return out


def place(sprite: np.ndarray, slot: dict, size: int) -> tuple[np.ndarray, list[int]]:
    ys, xs = np.nonzero(sprite[..., 3] > 20)
    x0, y0, x1, y1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
    crop = sprite[y0:y1, x0:x1]
    mw, mh = slot["max"]
    k = min(1.0, mw / crop.shape[1], mh / crop.shape[0])
    if k < 1.0:
        # 투명한 곳의 색이 가장자리에 번지지 않게 alpha 를 곱한 채로 줄인다
        im = Image.fromarray(crop, "RGBA").convert("RGBa")
        crop = np.array(im.resize((max(1, round(crop.shape[1] * k)), max(1, round(crop.shape[0] * k))), Image.LANCZOS).convert("RGBA"))
    h, w = crop.shape[:2]
    fx, fy = slot["foot"]
    left, top = int(round(fx - w / 2)), int(round(fy - h))
    canvas = np.zeros((size, size, 4), np.uint8)
    canvas[top:top + h, left:left + w] = crop
    return canvas, [left, top, left + w, top + h]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--thr", type=float, default=60)
    ap.add_argument("--only", default="")
    a = ap.parse_args()
    spec = json.loads((ART / "props.json").read_text())
    only = set(filter(None, a.only.split(",")))
    (OUT / "layers").mkdir(parents=True, exist_ok=True)
    have = []
    with tempfile.TemporaryDirectory() as t:
        # 빈 책상도 크로마키로 배경을 빼서 비교 기준으로
        chroma.run(ROOT / spec["base"], Path(t))
        base = align.load(Path(t) / Path(spec["base"]).name)
        size = base.shape[0]
        for pid, p in spec["props"].items():
            slot = spec["slots"][p["slot"]]
            if "max" in p:  # 소품별 크기 한도 (없으면 자리의 max)
                slot = {**slot, "max": p["max"]}
            dst = OUT / "layers" / f"{pid}.png"
            if only and pid not in only:
                if dst.exists():
                    have.append(pid)
                continue
            tmp = Path(t) / pid
            tmp.mkdir()
            if "sprite" in p:
                raw = ART / "sprite" / f"{p['sprite']}.png"
                if not raw.exists():
                    print(f"{pid}: {raw.relative_to(ROOT)} 없음, 건너뜀")
                    continue
                canvas, placed = place(key(raw), slot, size)
                Image.fromarray(canvas, "RGBA").save(dst, optimize=False, compress_level=3)
                have.append(pid)
                print(f"prop/{pid}: {p['slot']} {placed}")
                continue
            if "from" in p:
                f = p["from"]
                src, ref, box = prep(ROOT / f["src"], base, tmp), prep(ROOT / f["ref"], base, tmp), f["box"]
            else:
                raw = ART / "raw" / f"{p['src']}.png"
                if not raw.exists():
                    print(f"{pid}: {raw.relative_to(ROOT)} 없음, 건너뜀")
                    continue
                src, ref, box = prep(raw, base, tmp), base.astype(np.float32), slot["search"]
            full = extract(src, ref, a.thr)
            if "from" in p and "pick" in p["from"]:
                full = pick(peel(full, ref), p["from"]["pick"])
            sprite = cut(full, box)
            # 생성 그림의 소품이 search 상자 가장자리에 닿으면 잘린 것: search 를 넓혀야 한다
            sa = sprite[..., 3] > 40
            bx0, by0, bx1, by1 = box
            if "from" not in p and (sa[by0:by1, bx0].any() or sa[by0:by1, bx1 - 1].any() or sa[by0, bx0:bx1].any() or sa[by1 - 1, bx0:bx1].any()):
                print(f"경고 prop/{pid}: 소품이 상자 {box} 가장자리에서 잘림")
            if "from" not in p:  # 잘라 온 원래 소품은 주변 조각까지 불투명해지므로 그대로 둔다
                sprite = solid(sprite)
            canvas, placed = place(sprite, slot, size)
            Image.fromarray(canvas, "RGBA").save(dst, optimize=False, compress_level=3)
            have.append(pid)
            print(f"prop/{pid}: {p['slot']} {placed}")

    lines = ["# art/scripts/make_props.py 가 art/_props/props.json 에서 만든다. src/desk.rs"]
    for pid, p in spec["props"].items():
        if pid not in have:
            continue
        lines += ["", "[[prop]]", f"id = {toml_val(pid)}", f"title = {toml_val(p['title'])}", f"slot = {toml_val(p['slot'])}",
                  f"foot = {toml_val(spec['slots'][p['slot']]['foot'])}"]
        if "tap_ms" in p:
            lines.append(f"tap_ms = {p['tap_ms']}")
        for m in p.get("tap", []):
            lines += ["", "[[prop.tap]]"] + [f"{k} = {toml_val(v)}" for k, v in m.items()]
    for j in spec["jobs"]:
        items = [i for i in j["items"] if i in have]
        lines += ["", "[[job]]", f"id = {toml_val(j['id'])}", f"title = {toml_val(j['title'])}", f"items = {toml_val(items)}"]
    (OUT / "props.toml").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{(OUT / 'props.toml').relative_to(ROOT)}: 소품 {len(have)}개, 직군 {len(spec['jobs'])}개")


if __name__ == "__main__":
    main()
