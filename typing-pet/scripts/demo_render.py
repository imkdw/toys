"""동작 영상을 오프스크린 렌더로 만든다 (화면 녹화가 안 될 때, 예: 화면 잠김).

typing-pet --render-timeline 으로 뽑은 프레임(실제 앱과 같은 상태 머신/연출/합성)에
입력 상황과 머리 상태 자막을 붙여 mp4 로 만든다. 시나리오는 src/lib.rs 의 timeline_inputs().

사용법: demo_render.py <프레임 폴더> <출력 mp4> [--fps 60]
"""

import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

FONT = "/System/Library/Fonts/AppleSDGothicNeo.ttc"
HEAD = {"normal": "기본", "excited": "신남", "dizzy": "빙글 안경 + 귀 흔들기", "overheat": "과열", "sleepy": "졸림",
        "sunglasses": "선글라스"}
PAW_R = {"up": "", "down": "오른발 누름", "center": "오른발 가운데", "mouse": "오른발 마우스", "mouse_click": "마우스 클릭"}


def phase(ms: int) -> str:
    if 300 <= ms < 8000:
        return "빠르게 타이핑 중 (초당 약 12키)"
    if 8000 <= ms < 8200:
        return "타이핑 멈춤"
    if 8200 <= ms < 10300:
        return "마우스 이동 / 클릭"
    if 10300 <= ms < 13500:
        return "가만히 있음 (점수가 식는 중)"
    if 13500 <= ms < 14500:
        return "같은 키(T) 10연타"
    if 15000 <= ms < 17500:
        return "천천히 타이핑"
    return "가만히 있음"


def main() -> None:
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    fps = int(sys.argv[sys.argv.index("--fps") + 1]) if "--fps" in sys.argv else 60
    tl = json.loads((src / "timeline.json").read_text())
    work = out.parent / "composed"
    work.mkdir(parents=True, exist_ok=True)
    big = ImageFont.truetype(FONT, 30, index=6)
    small = ImageFont.truetype(FONT, 22, index=4)
    W, H = 960, 720
    bg = Image.new("RGBA", (W, H))
    d = ImageDraw.Draw(bg)
    for y in range(H):  # 바탕화면 느낌의 세로 그라데이션
        t = y / H
        d.line([(0, y), (W, y)], fill=(int(120 + 60 * t), int(170 + 40 * t), int(220 - 10 * t), 255))
    for i, s in enumerate(tl):
        f = Image.open(src / f"{i:05d}.png").convert("RGBA")
        im = bg.copy()
        im.alpha_composite(f, ((W - f.width) // 2, 70))
        dr = ImageDraw.Draw(im)
        st = s["state"]
        head = HEAD.get(st["head"], st["head"]) + (" + 제스처" if st.get("pose") else "")
        paws = " / ".join(x for x in ["왼발 누름" if st["paw_l"] == "down" else "", PAW_R.get(st["paw_r"], "")] if x)
        dr.rounded_rectangle((20, 16, W - 20, 60), 12, fill=(255, 255, 255, 215))
        dr.text((36, 21), f"입력: {phase(s['ms'])}", font=big, fill=(40, 30, 25, 255))
        dr.rounded_rectangle((20, H - 74, W - 20, H - 20), 12, fill=(255, 255, 255, 215))
        dr.text((36, H - 66), f"머리: {head}    {paws}", font=big, fill=(40, 30, 25, 255))
        dr.text((W - 210, H - 60), f"{s['ms'] / 1000:5.2f}s  점수 {s['score']:4.1f}", font=small, fill=(90, 80, 75, 255))
        im.convert("RGB").save(work / f"{i:05d}.png")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-framerate", str(fps), "-i", str(work / "%05d.png"),
                    "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", "-movflags", "+faststart", str(out)], check=True)
    gif = out.with_suffix(".gif")
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(out), "-vf",
                    "fps=20,scale=480:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=160[p];[b][p]paletteuse", str(gif)], check=True)
    print(out, gif)


if __name__ == "__main__":
    main()
