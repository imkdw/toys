"""라벨(text 또는 content-desc)로 노드를 눌러 본다. 사용: tap.py <라벨>"""
import os
import sys

import uiautomator2 as u2

d = u2.connect(os.environ.get("ANDROID_SERIAL"))
label = sys.argv[1]
for sel in (d(text=label), d(description=label)):
    if sel.exists:
        sel.click()
        print("clicked", label)
        break
else:
    print("not found", label)
    sys.exit(1)
