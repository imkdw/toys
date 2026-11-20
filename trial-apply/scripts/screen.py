"""현재 화면의 텍스트/설명 노드를 bounds 와 함께 출력한다 (셀렉터 확인용)."""
import os
import sys
import xml.etree.ElementTree as ET

import uiautomator2 as u2

d = u2.connect(os.environ.get("ANDROID_SERIAL"))
xml = d.dump_hierarchy()
if len(sys.argv) > 1:
    open(sys.argv[1], "w").write(xml)
for n in ET.fromstring(xml).iter("node"):
    t = n.get("text") or n.get("content-desc")
    if t:
        flags = "".join(k[0] for k in ("clickable", "selected", "enabled") if n.get(k) == "true")
        print(n.get("bounds"), n.get("class").split(".")[-1], flags, repr(t[:80]))
