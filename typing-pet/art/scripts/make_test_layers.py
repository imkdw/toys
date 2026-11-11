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


def scene(head="normal", paw_l="up", paw_r="up", gesture=False, startle=False, clean=False) -> Image.Image:
    im = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    if clean:
        # 캐릭터를 지운 책상 (base_clean): 머리/앞발 자리를 비워 둔다
        d.ellipse((40, 300, 472, 470), fill=(240, 170, 90, 255), outline=LINE, width=6)
        d.rounded_rectangle((120, 350, 360, 420), 10, fill=(250, 250, 250, 255), outline=LINE, width=4)
        d.rectangle((380, 360, 440, 410), fill=(230, 220, 200, 255), outline=LINE, width=4)
        return im
    # 머리 (책상 뒤)
    color = {"excited": (255, 220, 120, 255), "dizzy": (200, 255, 200, 255), "dizzy_wiggle": (200, 255, 200, 255),
             "overheat": (255, 90, 90, 255), "sleepy": (210, 210, 230, 255), "alt_a": (180, 230, 255, 255),
             "alt_b": (255, 190, 230, 255)}.get(head, CREAM)
    if startle:
        color = (255, 255, 160, 255)
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
    if startle:
        # 깜짝: 두 앞발을 번쩍 든 자세
        paw(140, 250, (255, 255, 160, 255))
        paw(372, 250, (255, 255, 160, 255))
        return im
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
    # behavior 엔진 테스트용 (계획 5.2): 머리 바꾸기 2장, 전체 프레임 1장, 소품, 캐릭터를 지운 책상, 몸 마스크
    for h in ["alt_a", "alt_b"]:
        save(root, f"head/{h}", scene(head=h))
    save(root, "full/startle", scene(startle=True))
    save(root, "base_clean", scene(clean=True))
    for name, draw in [
        ("bubble", lambda d: (d.rounded_rectangle((330, 40, 470, 120), 18, fill=(255, 255, 255, 255), outline=LINE, width=5),
                              d.polygon([(350, 115), (330, 150), (380, 118)], fill=(255, 255, 255, 255)))),
        ("drop", lambda d: d.ellipse((330, 190, 350, 220), fill=(120, 190, 255, 255), outline=LINE, width=3)),
        ("note", lambda d: (d.ellipse((90, 90, 120, 115), fill=LINE), d.rectangle((114, 40, 120, 104), fill=LINE))),
    ]:
        im = Image.new("RGBA", (C, C), (0, 0, 0, 0))
        draw(ImageDraw.Draw(im))
        save(root, f"fx/{name}", im)
    # 플래시: 캐릭터 실루엣 모양 흰색 (slice.py 가 실제 캐릭터에도 같은 방식으로 만든다)
    a = scene().getchannel("A")
    white = Image.new("RGBA", (C, C), (255, 255, 255, 255))
    white.putalpha(a)
    save(root, "fx/white", white)
    # 마스크: 서로 겹치지 않고, 경계는 단색(배경/책상 면)을 지나간다
    save(root, "mask/head", mask([("rectangle", (100, 25, 412, 292))]).convert("RGBA"))
    save(root, "mask/paw_l", mask([("rectangle", (125, 294, 228, 412))]).convert("RGBA"))
    save(root, "mask/paw_r", mask([("rectangle", (226, 294, 450, 418))]).convert("RGBA"))
    # 몸 마스크: 머리 + 앞발 자리를 넓힌 것 (slice.py 의 mask/body 와 같은 뜻)
    save(root, "mask/body", mask([("rectangle", (90, 20, 460, 370))]).convert("RGBA"))
    # 앞발 마스크를 키보드 윗줄(y=370)에서 위(몸과 같이 움직임)/아래(제자리)로 나눈 것
    for g, (x0, y0, x1, y1) in {"paw_l": (125, 294, 228, 412), "paw_r": (226, 294, 450, 418)}.items():
        save(root, f"mask/{g}_top", mask([("rectangle", (x0, y0, x1, 370))]).convert("RGBA"))
        save(root, f"mask/{g}_low", mask([("rectangle", (x0, 370, x1, y1))]).convert("RGBA"))

    (root / "manifest.toml").write_text(
        """# 검증용 가짜 레이어 세트. art/scripts/make_test_layers.py 로 만든다.
name = "_test"
canvas = 512
layers = [
  "base",
  "head/normal", "head/excited", "head/dizzy", "head/dizzy_wiggle", "head/overheat", "head/sleepy", "head/sunglasses",
  "paw_l/up", "paw_l/down",
  "paw_r/up", "paw_r/down", "paw_r/center", "paw_r/mouse", "paw_r/mouse_click",
  "head/alt_a", "head/alt_b",
  "base_clean",
  "fx/sunglasses", "fx/bubble", "fx/drop", "fx/note", "fx/white",
  "full/gesture", "full/startle",
  "mask/head", "mask/paw_l", "mask/paw_r", "mask/body",
  "mask/paw_l_top", "mask/paw_l_low", "mask/paw_r_top", "mask/paw_r_low",
]
head_anchor = [256, 330]
body_anchor = [256, 420]

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
"paw_l/down" = [160, 392]
"paw_r/down" = [320, 392]
"paw_r/center" = [250, 395]

# 엔진 테스트용 가짜 behavior: 모든 cue/motion/fx 종류를 한 번씩 쓴다 (characters/README.md).
# 기존 단위 테스트가 쓰는 단계 전환(excited, dizzy, 선글라스)과 장면 cue 는 비워 둔다.

[[behavior]]
name = "t_shout"
on = "level:overheat"
duration_ms = 900
priority = 20

[[behavior.fx]]
layer = "fx/bubble"
kind = "pop_in"
anchor = [400, 120]
period_ms = 800
scale = [0.3, 1.0]

[[behavior.motion]]
target = "body"
kind = "hop"
pt = 6
period_ms = 450
count = 1

[[behavior]]
name = "t_shiver"
on = "level:overheat"
when = "hold"
duration_ms = 600
cooldown_s = 0.4

[[behavior.motion]]
target = "body"
kind = "tremble"
pt = 1.5
period_ms = 60

[[behavior.fx]]
layer = "fx/drop"
kind = "fall"
anchor = [340, 205]
period_ms = 600
rise_pt = 20

[[behavior]]
name = "t_startle"
on = "wake"
frames = ["full/startle"]
duration_ms = 800
cooldown_s = 5

[[behavior.motion]]
target = "body"
kind = "hop"
pt = 8
period_ms = 400
count = 1

[[behavior.fx]]
layer = "fx/drop"
kind = "pulse"
anchor = [340, 205]
period_ms = 400
scale = [0.6, 1.0]

[[behavior]]
name = "t_bounce"
on = "burst"
duration_ms = 700
cooldown_s = 2

[[behavior.motion]]
target = "body"
kind = "hop"
pt = 3
period_ms = 350
count = 2

[[behavior.motion]]
target = "head"
kind = "squash"
squash = 0.06
period_ms = 350
count = 2

[[behavior]]
name = "t_nod"
on = "key:enter"
duration_ms = 600

[behavior.head]
layer = "head/alt_a"

[[behavior.motion]]
target = "head"
kind = "nod"
pt = 3
period_ms = 500
count = 1

[[behavior]]
name = "t_twitch"
on = "ambient"
duration_ms = 900

[behavior.head]
frames = ["head/alt_a", "head/alt_b"]
frame_ms = 150

[[behavior.motion]]
target = "head"
kind = "sway"
pt = 2
period_ms = 450

[[behavior]]
name = "t_flash"
on = "key:escape"
duration_ms = 1000
chance = 0.5

[[behavior.fx]]
layer = "fx/white"
kind = "flash"
anchor = [256, 256]
period_ms = 300

[[behavior.fx]]
layer = "fx/note"
kind = "float"
anchor = [105, 100]
period_ms = 1000
rise_pt = 12
delay_ms = 100

[[behavior.motion]]
target = "fx:note"
kind = "sway"
pt = 3
period_ms = 500

[[behavior]]
name = "t_slump"
on = "level:sleepy"
duration_ms = 800

[[behavior.motion]]
target = "body"
kind = "squash"
squash = 0.05
period_ms = 800
count = 1
""",
        encoding="utf-8",
    )
    print(root)


if __name__ == "__main__":
    main()
