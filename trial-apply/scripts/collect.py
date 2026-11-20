"""목록을 끝까지 스크롤하며 상품명을 순서대로 모아 local/items.json 에 저장한다.

사용: collect.py [--more N]   (--more: 끝의 더보기를 N 번 눌러 더 불러온다)
"""
import sys
import time

sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1]))
from trial import ITEMS, any_label, cards, open_trial_tab, save_json, scroll_down, to_top  # noqa: E402

more = int(sys.argv[sys.argv.index("--more") + 1]) if "--more" in sys.argv else 0
open_trial_tab()
to_top()
names, stale = [], 0
while stale < (3 if names else 8):
    new = [n for n, _ in cards() if n not in names]
    names += new
    if any_label("더보기") is not None and not new:
        if more <= 0:
            break
        any_label("더보기").click()
        more -= 1
        time.sleep(2)
        stale = 0
        continue
    stale = 0 if new else stale + 1
    scroll_down()
save_json(ITEMS, names)
for i, n in enumerate(names, 1):
    print(i, n)
