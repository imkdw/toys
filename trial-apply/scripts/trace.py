"""신청 흐름을 단계별로 화면을 찍으며 따라간다 (디버그). 사용: trace.py <상품명일부> [--go]"""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from trial import DETAIL_APPLY, any_label, cards, d, nodes, on_list, open_trial_tab, scroll_down, to_top  # noqa: E402

from classify import classify  # noqa: E402
from trial import RESULTS, load_json  # noqa: E402

key = sys.argv[1]
RES = load_json(RESULTS, {})


def pending(n):
    return n not in RES and classify(n)[0] == "apply"
go = "--go" in sys.argv


def show(tag):
    print("---", tag, time.strftime("%H:%M:%S"))
    for lbl, b, a in nodes():
        if lbl and b[1] > 2000 and "알림" not in lbl and b[1] < 2928:
            print(" ", b, a.get("class", "").split(".")[-1], repr(lbl[:60]))
    titles = [lbl for lbl, b, a in nodes() if lbl in ("체험단 신청", "신청 완료하기", "체험단 신청이 완료되었습니다.")]
    print("  titles:", titles)


open_trial_tab()
to_top()
for _ in range(60):
    hit = [(n, b) for n, b in cards() if (key in n if key != "AUTO" else pending(n))
           and min(b[3], 2450) - max(b[1], 520) > 120]
    if hit:
        print("item:", hit[0][0])
        b = hit[0][1]
        d.click((b[0] + b[2]) // 2, (max(b[1], 520) + min(b[3], 2450)) // 2)
        break
    scroll_down()
else:
    sys.exit("not found")
time.sleep(3)
show("detail")
any_label(DETAIL_APPLY).click()
for i in range(4):
    time.sleep(1)
    show(f"after apply click +{i + 1}s")
if any_label("위시에 담고 신청하기"):
    any_label("위시에 담고 신청하기").click()
    time.sleep(2)
    show("after wish")
if go and any_label("신청 완료하기"):
    any_label("신청 완료하기").click()
    for i in range(6):
        time.sleep(1)
        show(f"after submit +{i + 1}s")
