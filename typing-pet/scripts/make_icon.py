#!/usr/bin/env python3
"""앱 아이콘 만들기: 우사기 레이어(신난 얼굴 + 키보드 치는 손)를 macOS 둥근 사각형 위에 올린다.

    python3 scripts/make_icon.py    -> target/icon/AppIcon.png (1024), target/icon/AppIcon.icns

캐릭터 레이어가 비공개라 결과물도 저장소에 두지 않는다 (target/ 아래). scripts/bundle.sh 가 번들 때마다 만든다.
"""
import shutil
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
LAYERS = ROOT / "characters/usagi/layers"
OUT = ROOT / "target/icon"
N = 1024
# Apple 아이콘 격자: 1024 캔버스 안에 824 둥근 사각형, 위아래 여백은 그림자 자리
BOX = 824
TOP = (N - BOX) // 2


def squircle(size: int, scale: int = 4) -> Image.Image:
    """초타원(|x|^5 + |y|^5 = 1) 마스크. 크게 그린 뒤 줄여서 가장자리를 부드럽게."""
    s = size * scale
    r = s / 2
    pts = []
    import math

    for i in range(720):
        t = 2 * math.pi * i / 720
        c, d = math.cos(t), math.sin(t)
        x = r + r * math.copysign(abs(c) ** 0.4, c)
        y = r + r * math.copysign(abs(d) ** 0.4, d)
        pts.append((x, y))
    m = Image.new("L", (s, s), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    return m.resize((size, size), Image.LANCZOS)


def gradient(size: int, top: tuple, bottom: tuple) -> Image.Image:
    g = Image.new("RGBA", (size, size))
    px = g.load()
    for y in range(size):
        t = y / (size - 1)
        col = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,)
        for x in range(size):
            px[x, y] = col
    return g


def character() -> Image.Image:
    im = Image.open(LAYERS / "base.png").convert("RGBA")
    for name in ["head/excited", "paw_l/up", "paw_r/up"]:
        im.alpha_composite(Image.open(LAYERS / f"{name}.png").convert("RGBA"))
    return im.crop(im.getbbox())


def main() -> None:
    mask = squircle(BOX)
    tile = gradient(BOX, (206, 232, 255), (168, 206, 246))

    ch = character()
    h = int(BOX * 0.86)
    w = round(ch.width * h / ch.height)
    ch = ch.resize((w, h), Image.LANCZOS)
    tile.alpha_composite(ch, ((BOX - w) // 2, BOX - h - int(BOX * 0.035)))
    tile.putalpha(ImageChops.multiply(tile.getchannel("A"), mask))

    icon = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    shadow = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    sm = Image.new("L", (N, N), 0)
    sm.paste(mask, (TOP, TOP + 12))
    shadow.putalpha(sm.point(lambda v: v * 0.35).filter(ImageFilter.GaussianBlur(18)))
    icon.alpha_composite(shadow)
    icon.alpha_composite(tile, (TOP, TOP))

    OUT.mkdir(parents=True, exist_ok=True)
    icon.save(OUT / "AppIcon.png")

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for s in [16, 32, 128, 256, 512]:
            icon.resize((s, s), Image.LANCZOS).save(iconset / f"icon_{s}x{s}.png")
            icon.resize((s * 2, s * 2), Image.LANCZOS).save(iconset / f"icon_{s}x{s}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "AppIcon.icns")], check=True)
    print(OUT / "AppIcon.icns")


if __name__ == "__main__":
    if not shutil.which("iconutil"):
        raise SystemExit("iconutil 없음 (macOS 에서 실행)")
    main()
