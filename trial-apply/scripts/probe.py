"""상품 하나를 열어 신청 버튼 > (위시) > 옵션 드롭다운까지 가서 화면을 덤프한다. 신청은 하지 않는다."""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from trial import DETAIL_APPLY, any_label, cards, d, nodes, on_list, scroll_down, to_top  # noqa: E402

from trial import RESULTS, load_json, open_trial_tab  # noqa: E402

key = sys.argv[1]
RES = load_json(RESULTS, {})


def want(n):
    if key == "SIZE":
        return RES.get(n, {}).get("status") == "맞는 사이즈 없음"
    return key in n


def show(tag):
    print("---", tag)
    for lbl, b, a in nodes():
        if lbl and b[1] > 300 and "알림" not in lbl:
            print(b, a.get("class", "").split(".")[-1], a.get("clickable"), repr(lbl[:70]))


open_trial_tab()
if on_list():
    for _ in range(80):
        hit = [(n, b) for n, b in cards() if want(n) and min(b[3], 2450) - max(b[1], 520) > 120]
        if hit:
            print("item:", hit[0][0])
            b = hit[0][1]
            d.click((b[0] + b[2]) // 2, (max(b[1], 520) + min(b[3], 2450)) // 2)
            break
        scroll_down()
    time.sleep(2.5)
if any_label(DETAIL_APPLY):
    any_label(DETAIL_APPLY).click()
    time.sleep(2)
if any_label("위시에 담고 신청하기"):
    show("wish")
    any_label("위시에 담고 신청하기").click()
    time.sleep(2)
show("sheet")
dd = any_label("옵션을 선택해주세요")
if dd:
    dd.click()
    time.sleep(1.5)
    show("dropdown")
    import xml.etree.ElementTree as ET
    for n in ET.fromstring(d.dump_hierarchy()).iter("node"):
        if n.get("scrollable") == "true":
            print("SCROLLABLE", n.get("class"), n.get("bounds"), len(list(n.iter("node"))))
