"""레이어 조합 시트와 GIF 를 만든다 (PLAN 3.5-6, 6.5).

앱의 오프스크린 렌더러(typing-pet --render-all / --render-timeline)로 그리므로 실제 앱과 같은 합성 결과다.
- sheet.png: 모든 상태 조합 (머리 6 x 왼발 2 x 오른발 5 + 귀 흔들기 + 제스처)
- timeline.gif: 참고 영상의 시간순 동작을 흉내 낸 시나리오 (빠른 타이핑 -> 과열 -> 제스처 -> 마우스 -> 선글라스)
- paws.gif: base 와 앞발 변형을 번갈아 (경계 확인용)

사용법: preview.py [--name usagi | --character-dir <폴더>] -o <출력 폴더> [--size 240]
"""

import argparse
import json
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]


def app(args: list[str]) -> None:
    exe = ROOT / "target/debug/typing-pet"
    subprocess.run(["cargo", "build", "-q", "--bin", "typing-pet"], cwd=ROOT, check=True)
    subprocess.run([str(exe), *args], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)


def on_bg(im: Image.Image, color=(205, 225, 240, 255)) -> Image.Image:
    bg = Image.new("RGBA", im.size, color)
    bg.alpha_composite(im.convert("RGBA"))
    return bg


def sheet(src: Path, out: Path, tile: int) -> None:
    files = sorted(p for p in src.glob("*.png"))
    cols = 10
    rows = (len(files) + cols - 1) // cols
    s = Image.new("RGBA", (cols * tile, rows * (tile + 14)), (255, 255, 255, 255))
    d = ImageDraw.Draw(s)
    for i, f in enumerate(files):
        x, y = (i % cols) * tile, (i // cols) * (tile + 14)
        s.paste(on_bg(Image.open(f).resize((tile, tile), Image.LANCZOS)), (x, y + 14))
        label = f.stem.replace("head-", "").replace("paw_l-", "L:").replace("paw_r-", "R:").replace("__", " ")
        d.text((x + 2, y + 1), label[:34], fill=(0, 0, 0, 255))
    s.save(out)


def gif(frames: list[Image.Image], out: Path, ms: int) -> None:
    fr = [on_bg(f).convert("P", palette=Image.ADAPTIVE, colors=255) for f in frames]
    fr[0].save(out, save_all=True, append_images=fr[1:], duration=ms, loop=0, disposal=2)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", default="usagi")
    ap.add_argument("--character-dir", type=Path)
    ap.add_argument("-o", "--out", type=Path, required=True)
    ap.add_argument("--size", type=int, default=240)
    ap.add_argument("--fps", type=int, default=20)
    a = ap.parse_args()
    cdir = a.character_dir or ROOT / "characters" / a.name
    a.out.mkdir(parents=True, exist_ok=True)
    combos, tl = a.out / "combos", a.out / "timeline"
    for d in (combos, tl):
        if d.exists():
            for f in d.glob("*"):
                f.unlink()
    app(["--character-dir", str(cdir), "--render-all", str(combos), "--size", str(a.size)])
    sheet(combos, a.out / "sheet.png", 160)
    app(["--character-dir", str(cdir), "--render-timeline", str(tl), "--size", str(a.size), "--fps", str(a.fps), "--secs", "18"])
    frames = [Image.open(f) for f in sorted(tl.glob("*.png"))]
    gif(frames, a.out / "timeline.gif", 1000 // a.fps)
    # 앞발 경계 확인: up 과 각 앞발 자세를 번갈아
    paw = [combos / "head-normal__paw_l-up__paw_r-up.png"]
    for n in ["paw_l-down__paw_r-up", "paw_l-up__paw_r-down", "paw_l-up__paw_r-center", "paw_l-up__paw_r-mouse", "paw_l-up__paw_r-mouse_click"]:
        paw += [combos / f"head-normal__{n}.png", combos / "head-normal__paw_l-up__paw_r-up.png"]
    gif([Image.open(p) for p in paw], a.out / "paws.gif", 350)
    summary = json.loads((tl / "timeline.json").read_text())
    heads = []
    for s in summary:
        h = s["state"]["head"] + (f"+{s['state']['pose']}" if s["state"]["pose"] else "")
        if not heads or heads[-1][1] != h:
            heads.append((s["ms"], h))
    print("머리 변화:", " -> ".join(f"{ms / 1000:.1f}s {h}" for ms, h in heads))
    print(f"{a.out}: sheet.png, timeline.gif, paws.gif")


if __name__ == "__main__":
    main()
