"""책상 소품(직군 장비, 캐릭터 소품)을 생성 그림에서 잘라 characters/_props/ 를 만든다.

art/_props/props.json 을 읽는다.
- "src": art/_props/raw/<이름>.png. 빈 책상(art/_desk/desk_empty.png)에 소품 하나를 그려 넣은 그림.
  빈 책상과 달라진 덩어리를 소품 자리의 search 상자 안에서 찾아 자른다.
- "from": {"src", "ref", "box"}. 기존 캐릭터 그림에서 원래 소품을 잘라 온다 (ref 는 그 소품만 없는 그림).

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


def place(sprite: np.ndarray, slot: dict, size: int) -> tuple[np.ndarray, list[int]]:
    ys, xs = np.nonzero(sprite[..., 3] > 20)
    x0, y0, x1, y1 = xs.min(), ys.min(), xs.max() + 1, ys.max() + 1
    crop = sprite[y0:y1, x0:x1]
    mw, mh = slot["max"]
    k = min(1.0, mw / crop.shape[1], mh / crop.shape[0])
    if k < 1.0:
        im = Image.fromarray(crop, "RGBA")
        crop = np.array(im.resize((max(1, round(crop.shape[1] * k)), max(1, round(crop.shape[0] * k))), Image.LANCZOS))
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
            dst = OUT / "layers" / f"{pid}.png"
            if only and pid not in only:
                if dst.exists():
                    have.append(pid)
                continue
            tmp = Path(t) / pid
            tmp.mkdir()
            if "from" in p:
                f = p["from"]
                src, ref, box = prep(ROOT / f["src"], base, tmp), prep(ROOT / f["ref"], base, tmp), f["box"]
            else:
                raw = ART / "raw" / f"{p['src']}.png"
                if not raw.exists():
                    print(f"{pid}: {raw.relative_to(ROOT)} 없음, 건너뜀")
                    continue
                src, ref, box = prep(raw, base, tmp), base.astype(np.float32), slot["search"]
            sprite = cut(extract(src, ref, a.thr), box)
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
