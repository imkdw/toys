"""공용 책상 그림을 만든다 (art/_desk/). 책상 소품을 캐릭터 그림에서 떼어 내기 위한 준비.

- desk_empty.png: 캐릭터도 소품도 없는 책상 (키보드/마우스만). 우사기 base_clean 을 편집한 생성 그림
  (art/usagi/prompts/71_desk_empty.txt) 을 그대로 복사한다. make_props.py 가 소품을 잘라 낼 때 비교 기준.
- item_mask.png: 예전 소품(연필꽂이, 과자, 별 쿠션, 콜라)이 있던 자리. 소품마다 경계 상자를 넉넉히 칠한다.
  slice.py 가 picks.json 의 "desk_empty" 가 있는 캐릭터에서 이 자리를 빈 책상으로 바꾼다.
- usagi_noitems.png: 우사기 base 에서 소품만 지운 그림. 새 캐릭터 base 를 만들 때 입력(-i)으로 쓴다
  (책상 구도는 같고 소품은 처음부터 없다). "_erase" 상자의 자국도 지운다.

사용법: make_desk.py [--empty desk_empty_1] [--clean base_clean_1]
"""

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "art" / "_desk"
RAW = ROOT / "art" / "usagi" / "raw"
# 예전 소품 경계 상자 (x, y, w, h): 연필꽂이, 콜라, 과자, 별 쿠션. 세 캐릭터 모두 몇 px 안에서 같다
ITEMS = [(122, 615, 81, 171), (817, 619, 82, 169), (204, 726, 92, 72), (128, 787, 147, 108)]
PAD = 10


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--empty", default="desk_empty_1")
    ap.add_argument("--clean", default="base_clean_1")
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    empty = np.array(Image.open(RAW / f"{a.empty}.png").convert("RGB"))
    Image.fromarray(empty).save(OUT / "desk_empty.png")

    clean = np.array(Image.open(RAW / f"{a.clean}.png").convert("RGB")).astype(int)
    diff = (np.abs(clean - empty.astype(int)).sum(-1) > 60).astype(np.uint8)
    m = np.zeros(empty.shape[:2], np.float32)
    for x, y, w, h in ITEMS:
        sub = diff[y - PAD:y + h + PAD, x - PAD:x + w + PAD]
        print(f"소품 상자 {(x, y, w, h)}: 달라진 픽셀 {int(sub.sum())}")
        m[y - PAD:y + h + PAD, x - PAD:x + w + PAD] = 1
    Image.fromarray((m * 255).astype(np.uint8)).save(OUT / "item_mask.png")

    mf = cv2.GaussianBlur(m, (9, 9), 3)[..., None]
    base = np.array(Image.open(RAW / "base_final.png").convert("RGB")).astype(np.float32)
    out = (base * (1 - mf) + empty.astype(np.float32) * mf).clip(0, 255).astype(np.uint8)
    for x0, y0, x1, y1 in json.loads((ROOT / "art/usagi/picks.json").read_text()).get("_erase", []):
        reg = out[y0:y1, x0:x1].astype(int)
        lum = reg.sum(-1)
        skin = np.median(reg[lum >= np.percentile(lum, 60)], axis=0)
        reg[np.abs(reg - skin).sum(-1) > 6] = skin
        out[y0:y1, x0:x1] = reg
    Image.fromarray(out).save(OUT / "usagi_noitems.png")
    print(f"{OUT.relative_to(ROOT)}: desk_empty.png, item_mask.png, usagi_noitems.png")


if __name__ == "__main__":
    main()
