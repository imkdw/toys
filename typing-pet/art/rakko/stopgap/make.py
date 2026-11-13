"""codex 이미지 생성 한도에 걸려 만든 임시 레이어 (art/rakko/raw/*_pil.png).

base_final.png 를 손대지 않고 코드로만 편집한다:
- 앞발 누름: 짙은 갈색 앞발 덩어리를 아래로 옮기고 빈자리를 인페인트
- 말풍선/번개/베기 빛줄기/입김: PIL 로 그림 (extract_fx.py 가 base_final 과의 차이만 잘라 fx/ 로 만든다)
생성 한도가 풀리면 prompts/ 로 다시 생성해 picks.json 을 바꾼다 (art/rakko/BEHAVIORS.md "남은 일").
"""
import numpy as np, cv2, math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

RAW = Path(__file__).resolve().parents[1] / "raw"
base = Image.open(RAW / "base_final.png").convert("RGB")
B = np.array(base)
LINE = (58, 42, 37)
FONT = "/System/Library/Fonts/AppleSDGothicNeo.ttc"


def paw_mask():
    d = (np.abs(B.astype(int) - [70, 38, 36]).max(-1) < 40).astype(np.uint8)
    d = cv2.morphologyEx(d, cv2.MORPH_OPEN, np.ones((9, 9), np.uint8))
    n, lab, st, c = cv2.connectedComponentsWithStats(d, 8)
    out = {}
    for i in range(1, n):
        x, y, w, h, a = st[i]
        if a > 2000 and y > 600:
            m = (lab == i).astype(np.uint8)
            m = cv2.dilate(m, np.ones((9, 9), np.uint8))  # 외곽선 포함
            out["l" if x < 512 else "r"] = m
    return out


def press_paw(m, dy, name):
    """앞발을 아래로 dy 만큼 늘여 키보드를 누른 모양으로 다시 그린다 (옛 앞발을 그대로 덮으므로 인페인트가 필요 없다)."""
    ys, xs = np.nonzero(m)
    x0, x1, y0, y1 = xs.min() + 3, xs.max() - 3, ys.min() + 3, ys.max() - 3
    im = Image.fromarray(B.copy())
    d = ImageDraw.Draw(im)
    d.ellipse((x0 - 2, y0, x1 + 2, y1 + dy), fill=(74, 35, 34), outline=LINE, width=4)
    im.save(RAW / f"{name}.png")


def font(sz):
    return ImageFont.truetype(FONT, sz, index=6)  # Bold


def bubble(text, name, box=(660, 90, 1010, 290), tail=(760, 330)):
    im = base.copy()
    d = ImageDraw.Draw(im)
    x0, y0, x1, y1 = box
    # 꼬리
    cx = (x0 + x1) / 2
    d.polygon([(cx - 60, y1 - 30), (cx - 10, y1 - 20), tail], fill="white", outline=LINE)
    d.line([(cx - 60, y1 - 26), tail, (cx - 10, y1 - 18)], fill=LINE, width=5, joint="curve")
    d.ellipse(box, fill="white", outline=LINE, width=5)
    f = font(58 if len(text) <= 6 else 46)
    tb = d.textbbox((0, 0), text, font=f)
    d.text((cx - (tb[2] - tb[0]) / 2, (y0 + y1) / 2 - (tb[3] - tb[1]) / 2 - tb[1]), text, font=f, fill=LINE)
    im.save(RAW / f"{name}.png")


def bolt(name, at=(860, 300)):
    im = base.copy().convert("RGBA")
    glow = Image.new("RGBA", im.size, (0, 0, 0, 0))
    g = ImageDraw.Draw(glow)
    x, y = at
    pts = [(x - 20, y - 110), (x + 45, y - 110), (x + 10, y - 40), (x + 55, y - 40), (x - 35, y + 90), (x - 5, y + 5), (x - 45, y + 5)]
    im = Image.alpha_composite(im, glow)
    d = ImageDraw.Draw(im)
    d.polygon(pts, fill=(255, 228, 92), outline=LINE)
    d.line(pts + [pts[0]], fill=LINE, width=4, joint="curve")
    inner = [(x - 8, y - 95), (x + 22, y - 95), (x - 2, y - 30), (x + 25, y - 30), (x - 15, y + 40), (x - 2, y - 8), (x - 25, y - 8)]
    d.polygon(inner, fill=(255, 255, 235))
    im.convert("RGB").save(RAW / f"{name}.png")


def slash(name):
    """베기 빛줄기: 크로마키에 초록이 섞이지 않게 흐림 없이 단색 띠 (연노랑 테두리 + 흰 심)."""
    im = base.copy()
    d = ImageDraw.Draw(im)
    a, b = (990, 90), (60, 560)
    d.line([a, b], fill=LINE, width=40)
    d.line([a, b], fill=(255, 238, 120), width=32)
    d.line([a, b], fill=(255, 253, 225), width=12)
    out = np.array(im)
    bg = (B[..., 1] > 200) & (B[..., 0] < 90) & (B[..., 2] < 90)
    out[~bg] = B[~bg]
    Image.fromarray(out).save(RAW / f"{name}.png")


def puffs(name):
    """입김: 볼 옆의 흰 구름 두 덩이 (흐림 없이, 얇은 회색 외곽선)."""
    im = base.copy()
    d = ImageDraw.Draw(im)
    for cx, cy in [(205, 520), (820, 520)]:
        circles = [(0, 0, 34), (-30, 14, 24), (28, 16, 26), (6, -24, 22)]
        for ox, oy, r in circles:
            d.ellipse((cx + ox - r - 3, cy + oy - r - 3, cx + ox + r + 3, cy + oy + r + 3), fill=(190, 190, 190))
        for ox, oy, r in circles:
            d.ellipse((cx + ox - r, cy + oy - r, cx + ox + r, cy + oy + r), fill=(255, 255, 255))
    out = np.array(im)
    bg = (B[..., 1] > 200) & (B[..., 0] < 90) & (B[..., 2] < 90)
    out[~bg] = B[~bg]
    Image.fromarray(out).save(RAW / f"{name}.png")


if __name__ == "__main__":
    pm = paw_mask()
    press_paw(pm["l"], 26, "left_down_pil")
    press_paw(pm["r"], 26, "right_down_pil")
    bubble("늘 먹던 걸로!", "behavior_order_bubble_pil")
    bubble("좋았어!", "behavior_joa_bubble_pil")
    bubble("빨리 가야겠군!", "behavior_hurry_bubble_pil")
    bubble("흠. 감사.", "behavior_thanks_bubble_pil")
    bubble("흠...", "behavior_hmm_bubble_pil")
    bolt("behavior_bolt_pil")
    slash("behavior_slash_pil")
    puffs("behavior_puff_pil")
    print("ok")
