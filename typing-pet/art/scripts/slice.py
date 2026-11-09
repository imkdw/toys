"""마스크로 레이어를 잘라 characters/<이름>/layers/ 에 저장하고 manifest.toml 을 만든다 (PLAN 3.5-5).

규칙은 단순하게 "마스크 안은 변형 그림, 밖은 base" 다. 앱은 마스크가 있는 레이어를
"마스크 안을 교체" 로 그리므로(base 의 귀도 지울 수 있음) 레이어에는 변형 그림을, 마스크는 따로 저장한다.
경계는 페더링(가우시안)으로 섞는다.

사용법: slice.py [--name usagi] [--feather 6]
"""

import argparse
import json
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "art/work"
MASKS = ROOT / "art/masks"
GROUP_OF = {"head": "head", "paw_l": "paw_l", "paw_r": "paw_r"}


def feathered(name: str, feather: int) -> np.ndarray:
    m = np.array(Image.open(MASKS / f"{name}.png").convert("L")).astype(np.float32) / 255
    k = feather * 2 + 1
    return cv2.GaussianBlur(m, (k, k), feather / 2)


def save_layer(out: Path, name: str, rgba: np.ndarray) -> None:
    p = out / "layers" / f"{name}.png"
    p.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgba, "RGBA").save(p, optimize=False, compress_level=3)


def job(args):
    out, name, src, mask = args
    rgba = np.array(Image.open(src).convert("RGBA"))
    if mask is not None:
        # 마스크 밖은 쓰이지 않으므로 비워서 파일을 작게
        rgba[mask <= 0.0] = 0
    save_layer(out, name, rgba)
    return name


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", default="usagi")
    ap.add_argument("--feather", type=int, default=6)
    a = ap.parse_args()
    out = ROOT / "characters" / a.name
    picks = json.loads((ROOT / "art/picks.json").read_text())
    geo = json.loads((WORK / "geometry.json").read_text())

    masks = {g: feathered(g, a.feather) for g in ["head", "paw_l", "paw_r"]}
    # 머리/앞발 마스크가 겹치는 정도 확인 (겹치면 경계에서 이전 자세가 비쳐 보인다)
    for x, y in [("head", "paw_l"), ("head", "paw_r"), ("paw_l", "paw_r")]:
        ov = np.minimum(masks[x], masks[y]).max()
        print(f"마스크 겹침 {x}/{y}: 최대 {ov:.3f}")
    for g, m in masks.items():
        save_layer(out, f"mask/{g}", np.dstack([(m * 255).astype(np.uint8)] * 3 + [np.full(m.shape, 255, np.uint8)]))

    jobs = [(out, "base", WORK / "base.png", None), (out, "head/normal", WORK / "base.png", masks["head"]),
            (out, "paw_l/up", WORK / "base.png", masks["paw_l"]), (out, "paw_r/up", WORK / "base.png", masks["paw_r"])]
    for layer, raw in picks.items():
        if layer.startswith("_") or layer == "base":
            continue
        src = WORK / "matched" / f"{raw}.png"
        if not src.exists():
            print(f"없음, 건너뜀: {layer} <- {src.name}")
            continue
        group = layer.split("/")[0]
        jobs.append((out, layer, src, masks.get(GROUP_OF.get(group, ""), None)))
    with ProcessPoolExecutor() as ex:
        names = list(ex.map(job, jobs))

    # 선글라스 단독 (fx): 선글라스 영역만, 일반 over 합성
    if (MASKS / "sunglasses.png").exists() and "head/sunglasses" in picks:
        v = np.array(Image.open(WORK / "matched" / f"{picks['head/sunglasses']}.png").convert("RGBA")).astype(np.float32)
        m = feathered("sunglasses", 1)
        v[..., 3] *= m
        save_layer(out, "fx/sunglasses", v.astype(np.uint8))
        names.append("fx/sunglasses")
    names += ["mask/head", "mask/paw_l", "mask/paw_r"]

    order = ["base", "head/", "paw_l/", "paw_r/", "fx/", "full/", "mask/"]
    names.sort(key=lambda n: (next(i for i, p in enumerate(order) if n.startswith(p)), n))
    tips = "\n".join(f'"{k}" = [{v[0]}, {v[1]}]' for k, v in geo["tips"].items())
    layers = ",\n".join(f'  "{n}"' for n in names)
    ha = geo["head_anchor"]
    canvas = Image.open(WORK / "base.png").size[0]
    (out / "manifest.toml").write_text(
        f"""# art/scripts/slice.py 가 만든다. [timing] [anim] [keys] 를 넣으면 기본값을 덮어쓴다 (src/config.rs).
name = "{a.name}"
canvas = {canvas}
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
""",
        encoding="utf-8",
    )
    print(f"{out.relative_to(ROOT)}: 레이어 {len(names)}장")


if __name__ == "__main__":
    main()
