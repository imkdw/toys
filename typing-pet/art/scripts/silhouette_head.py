"""머리 마스크를 캐릭터 실루엣(책상 위 몸 전체)으로 넓힌다.

automask 는 머리 변형과 base 의 차이로 머리 마스크를 잡는다. 표정만 바뀌는 캐릭터(모몽가, 쿠리만쥬, 랏코)는
눈/입 언저리만 잡혀서 몸 마스크(mask/body)도 작아지고, hop/sway 때 귀/윤곽/꼬리가 제자리에 남는다.
그래서 base 와 빈 책상(base_clean) 의 차이로 실루엣을 구해 머리 마스크에 더하고, 앞발 마스크와는 겹치지 않게 뺀다.

사용법: art/.venv/bin/python art/scripts/silhouette_head.py --name momonga   (build.sh 한 번 돌린 뒤)
그 다음 build.sh --name <이름> 을 다시 돌린다 (--remask 를 주면 automask 가 이 마스크를 덮어쓴다).
"""
import argparse
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]


def disk(r: int) -> np.ndarray:
    return cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2 * r + 1, 2 * r + 1))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--name", required=True)
    ap.add_argument("--grow", type=int, default=8, help="실루엣을 넓히는 px (윤곽선 포함)")
    ap.add_argument("--paw-gap", type=int, default=14, help="앞발 마스크에서 띄우는 px")
    a = ap.parse_args()
    art = ROOT / "art" / a.name
    work, masks = art / "work", art / "masks"
    import json

    picks = json.loads((art / "picks.json").read_text())
    base = np.asarray(Image.open(work / "base.png").convert("RGBA")).astype(int)
    clean = np.asarray(Image.open(work / "matched" / f"{picks['base_clean']}.png").convert("RGBA")).astype(int)

    diff = (np.abs(base[..., :3] - clean[..., :3]).sum(2) > 60) & (base[..., 3] > 0)
    diff |= (base[..., 3] > 128) & (clean[..., 3] < 128)
    d = cv2.morphologyEx(diff.astype(np.uint8), cv2.MORPH_OPEN, np.ones((5, 5), np.uint8))
    n, lab, st, _ = cv2.connectedComponentsWithStats(d)
    big = int(np.argmax(st[1:, cv2.CC_STAT_AREA])) + 1
    cnts, _ = cv2.findContours((lab == big).astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    sil = np.zeros(d.shape, np.uint8)
    cv2.drawContours(sil, cnts, -1, 1, -1)

    head = cv2.dilate(sil, disk(a.grow)) > 0
    head |= np.asarray(Image.open(masks / "head.png").convert("L")) > 127
    paws = np.zeros_like(head)
    for g in ("paw_l", "paw_r"):
        paws |= np.asarray(Image.open(masks / f"{g}.png").convert("L")) > 0
    head &= ~(cv2.dilate(paws.astype(np.uint8), disk(a.paw_gap)) > 0)
    Image.fromarray((head * 255).astype(np.uint8)).save(masks / "head.png")
    print(f"{masks / 'head.png'}: {head.mean() * 100:.1f}% (실루엣 {sil.mean() * 100:.1f}%)")


if __name__ == "__main__":
    main()
