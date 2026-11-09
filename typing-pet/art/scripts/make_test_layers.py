"""검증용 가짜 레이어 세트(characters/_test/)를 만든다. 색 도형이라 공개해도 된다 (PLAN 6.7).

실제 캐릭터와 같은 구조로 만든다: 레이어는 "그 변형의 전체 장면", 마스크는 레이어 묶음이 바꾸는 영역.
레이어마다 색과 위치가 달라서 조합 시트에서 어떤 레이어가 그려졌는지 눈으로 바로 구분된다.
"""

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

C = 512
LINE = (80, 50, 30, 255)
CREAM = (250, 235, 200, 255)


def save(root: Path, name: str, im: Image.Image) -> None:
    p = root / "layers" / f"{name}.png"
    p.parent.mkdir(parents=True, exist_ok=True)
    im.save(p)


def scene(head="normal", paw_l="up", paw_r="up", gesture=False) -> Image.Image:
    im = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    # 머리 (책상 뒤)
    color = {"excited": (255, 220, 120, 255), "dizzy": (200, 255, 200, 255), "dizzy_wiggle": (200, 255, 200, 255),
             "overheat": (255, 90, 90, 255), "sleepy": (210, 210, 230, 255)}.get(head, CREAM)
    if head == "sleepy":
        d.ellipse((110, 150, 230, 195), fill=color, outline=LINE, width=6)
        d.ellipse((282, 150, 402, 195), fill=color, outline=LINE, width=6)
    elif head == "dizzy_wiggle":
        for i, c in enumerate([(255, 0, 0, 255), (255, 200, 0, 255), (0, 160, 255, 255)]):
            d.ellipse((168 + i * 10, 40, 208 + i * 10, 170), fill=c)
            d.ellipse((304 - i * 10, 40, 344 - i * 10, 170), fill=c)
    else:
        d.ellipse((190, 40, 230, 170), fill=color, outline=LINE, width=6)
        d.ellipse((282, 40, 322, 170), fill=color, outline=LINE, width=6)
    d.ellipse((150, 130, 362, 330), fill=color, outline=LINE, width=6)
    if head == "overheat":
        d.ellipse((198, 204, 228, 234), fill=(255, 255, 255, 255), outline=LINE, width=4)
        d.ellipse((284, 204, 314, 234), fill=(255, 255, 255, 255), outline=LINE, width=4)
    else:
        d.ellipse((205, 210, 221, 226), fill=(40, 30, 20, 255))
        d.ellipse((291, 210, 307, 226), fill=(40, 30, 20, 255))
    if head in ("dizzy", "dizzy_wiggle"):
        d.rectangle((185, 200, 327, 235), fill=(120, 120, 255, 255))
    if head == "sunglasses":
        d.rectangle((185, 200, 327, 235), fill=(10, 10, 10, 255))
    # 책상, 키보드, 패드
    d.ellipse((40, 300, 472, 470), fill=(240, 170, 90, 255), outline=LINE, width=6)
    d.rounded_rectangle((120, 350, 360, 420), 10, fill=(250, 250, 250, 255), outline=LINE, width=4)
    d.rectangle((380, 360, 440, 410), fill=(230, 220, 200, 255), outline=LINE, width=4)

    def paw(x, y, color):
        d.ellipse((x - 26, y - 22, x + 26, y + 22), fill=color, outline=LINE, width=5)

    paw(*({"up": (165, 320), "down": (160, 375)}[paw_l]), CREAM if paw_l == "up" else (255, 150, 150, 255))
    if gesture:
        paw(360, 245, (255, 200, 0, 255))
    else:
        pos = {"up": (315, 320), "down": (320, 375), "center": (250, 378), "mouse": (410, 370), "mouse_click": (410, 385)}[paw_r]
        col = {"up": CREAM, "down": (150, 150, 255, 255), "center": (150, 220, 255, 255), "mouse": (200, 150, 255, 255),
               "mouse_click": (255, 120, 255, 255)}[paw_r]
        paw(*pos, col)
    return im


def mask(shapes) -> Image.Image:
    m = Image.new("L", (C, C), 0)
    d = ImageDraw.Draw(m)
    for kind, box in shapes:
        getattr(d, kind)(box, fill=255)
    return m.filter(ImageFilter.GaussianBlur(3))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("-o", "--out", type=Path, default=Path("characters/_test"))
    root = ap.parse_args().out

    save(root, "base", scene())
    for h in ["normal", "excited", "dizzy", "dizzy_wiggle", "overheat", "sleepy", "sunglasses"]:
        save(root, f"head/{h}", scene(head=h))
    sg = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    ImageDraw.Draw(sg).rectangle((185, 200, 327, 235), fill=(10, 10, 10, 255))
    save(root, "fx/sunglasses", sg)
    for p in ["up", "down"]:
        save(root, f"paw_l/{p}", scene(paw_l=p))
    for p in ["up", "down", "center", "mouse", "mouse_click"]:
        save(root, f"paw_r/{p}", scene(paw_r=p))
    save(root, "full/gesture", scene(gesture=True))
    # 마스크: 서로 겹치지 않고, 경계는 단색(배경/책상 면)을 지나간다
    save(root, "mask/head", mask([("rectangle", (100, 25, 412, 292))]).convert("RGBA"))
    save(root, "mask/paw_l", mask([("rectangle", (125, 294, 228, 412))]).convert("RGBA"))
    save(root, "mask/paw_r", mask([("rectangle", (226, 294, 450, 418))]).convert("RGBA"))

    (root / "manifest.toml").write_text(
        """# 검증용 가짜 레이어 세트. art/scripts/make_test_layers.py 로 만든다.
name = "_test"
canvas = 512
layers = [
  "base",
  "head/normal", "head/excited", "head/dizzy", "head/dizzy_wiggle", "head/overheat", "head/sleepy", "head/sunglasses",
  "paw_l/up", "paw_l/down",
  "paw_r/up", "paw_r/down", "paw_r/center", "paw_r/mouse", "paw_r/mouse_click",
  "fx/sunglasses",
  "full/gesture",
  "mask/head", "mask/paw_l", "mask/paw_r",
]
head_anchor = [256, 330]

[masks]
head = "mask/head"
paw_l = "mask/paw_l"
paw_r = "mask/paw_r"

[tips]
"paw_l/down" = [160, 392]
"paw_r/down" = [320, 392]
"paw_r/center" = [250, 395]
""",
        encoding="utf-8",
    )
    print(root)


if __name__ == "__main__":
    main()
