"""체험단 자동 신청 공용 함수 (uiautomator2)."""
import json
import os
import time
import xml.etree.ElementTree as ET
from pathlib import Path

import uiautomator2 as u2

ROOT = Path(__file__).resolve().parent
LOCAL = ROOT / "local"
RESULTS = LOCAL / "results.json"
ITEMS = LOCAL / "items.json"

DETAIL_APPLY = "선물 체험단 신청하기"
CARD_LEFTS = (64, 736)

d = u2.connect(os.environ.get("ANDROID_SERIAL"))


class WrongScreen(RuntimeError):
    pass


def any_label(label: str, **kw):
    """text 또는 content-desc 가 label 인 노드."""
    for sel in (d(text=label, **kw), d(description=label, **kw)):
        if sel.exists:
            return sel
    return None


def wait_label(label: str, timeout: float = 5, **kw):
    end = time.time() + timeout
    while time.time() < end:
        sel = any_label(label, **kw)
        if sel is not None:
            return sel
        time.sleep(0.2)
    return None


def nodes():
    """현재 화면 노드 목록: (label, bounds(l,t,r,b), attrs)."""
    out = []
    for n in ET.fromstring(d.dump_hierarchy()).iter("node"):
        label = n.get("text") or n.get("content-desc") or ""
        b = n.get("bounds", "[0,0][0,0]").replace("][", ",").strip("[]").split(",")
        out.append((label, tuple(int(x) for x in b), n.attrib))
    return out


def on_list() -> bool:
    if d(description="공유").exists or d(text="공유").exists:  # 상세 페이지
        return False
    if d(description="체험단").exists:  # 상단 탭 줄은 스크롤해도 남는다
        return True
    return d(description="선물하기").exists and bool(cards())


def ensure_list(max_back: int = 3):
    for _ in range(max_back + 1):
        if on_list():
            return
        d.press("back")
        time.sleep(1.2)
    raise WrongScreen("목록 화면으로 돌아오지 못함")


def on_detail() -> bool:
    """상품 상세: 하단 공유 버튼 + 상품명: 노드."""
    return bool(d(description="공유").exists or d(text="공유").exists) and any(
        lbl.startswith("상품명:") for lbl, _, _ in nodes())


def session_expired() -> bool:
    return bool(d(textStartsWith="사용자 인증에 실패").exists or d(descriptionStartsWith="사용자 인증에 실패").exists)


def read_env_file() -> dict:
    """저장소 루트 .env 의 KEY=VALUE 줄을 읽는다 (git 에는 안 올라간다)."""
    env = Path(__file__).resolve().parent / ".env"
    if not env.exists():
        return {}
    out = {}
    for line in env.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            out[k.strip()] = v.strip().strip('"').strip("'")
    return out


def app_package() -> str:
    """대상 앱 패키지명. 저장소에 앱 이름을 남기지 않으려고 환경변수로 받는다."""
    pkg = os.environ.get("TRIAL_APP_PACKAGE") or read_env_file().get("TRIAL_APP_PACKAGE")
    if not pkg:
        raise SystemExit(".env 의 TRIAL_APP_PACKAGE 에 대상 앱 패키지명을 넣어 주세요")
    return pkg


def open_trial_tab():
    """메신저 앱 친구 탭 상단 선물하기 > 체험단 탭으로 들어간다."""
    if session_expired():  # 선물하기 세션 만료: 닫기 누르면 선물하기가 종료된다
        sel = d(description="닫기", className="android.view.View")
        (sel if sel.exists else d(text="닫기")).click()
        time.sleep(2)
    for _ in range(5):  # 상세/시트/신청 페이지에 있으면 탭 줄이 보일 때까지 뒤로
        if d(description="체험단").exists or not d(description="선물하기").exists:
            break
        d.press("back")
        time.sleep(1.2)
    if not (d(description="체험단").exists or d(description="선물하기", className="android.view.View").exists):
        d.app_start(app_package())
        time.sleep(2)
        btn = d(description="선물하기", className="android.widget.Button")
        if not btn.wait(timeout=3):
            for _ in range(3):  # 채팅 방 안이면 뒤로, 다른 탭이면 친구 탭으로
                friends = d(descriptionStartsWith="친구 탭")
                if friends.exists:
                    friends.click()
                    break
                d.press("back")
                time.sleep(1)
        if not btn.wait(timeout=5):
            raise WrongScreen("친구 탭의 선물하기 버튼이 없음")
        btn.click()
    if not d(description="홈").wait(timeout=10):
        raise WrongScreen("선물하기 화면이 열리지 않음")
    to_top()  # 탭 줄이 접혀 있으면 엉뚱한 곳이 눌린다
    tab = d(description="체험단")
    for _ in range(4):  # 상단 탭 줄은 가로 스크롤이라 오른쪽 끝 체험단이 가려져 있다
        if tab.exists:
            break
        d.shell("input swipe 1300 377 200 377 300")
        time.sleep(0.8)
    else:
        raise WrongScreen("선물하기에 체험단 탭이 없음")
    tb = tab.info["bounds"]
    d.click((tb["left"] + tb["right"]) // 2, (tb["top"] + tb["bottom"]) // 2)
    time.sleep(2.5)
    if not on_list():
        raise WrongScreen("체험단 탭을 열지 못함")


def cards():
    """화면에 보이는 상품 카드 [(name, bounds)] (중복 제거, 위에서 아래)."""
    seen, out = set(), []
    for label, b, a in nodes():
        if a.get("clickable") == "true" and b[0] in CARD_LEFTS and b[2] - b[0] == 640 and label:
            if label not in seen:
                seen.add(label)
                out.append((label, b))
    out.sort(key=lambda x: (x[1][1], x[1][0]))
    return out


def at_top() -> bool:
    for lbl, b, a in nodes():
        if lbl == "선물하기" and b[3] - b[1] > 150:  # 맨 위에서만 헤더가 온전히 보인다
            return True
    return False


def to_top():
    for _ in range(4):
        if at_top():
            return
        top = d(description="맨 위로")
        if top.exists:
            top.click()
        else:
            for _ in range(5):
                d.shell("input swipe 720 900 720 2600 150")
        time.sleep(1.5)
    if not at_top():
        raise WrongScreen("목록 맨 위로 못 감")


def scroll_down():
    """한 줄쯤 내린다. 웹뷰가 가끔 스와이프를 탭으로 받아 상세가 열리면 뒤로 간다."""
    d.shell("input swipe 720 2300 720 900 400")
    time.sleep(0.5)
    for _ in range(6):
        if on_list():
            return
        if on_detail():
            d.press("back")
        time.sleep(0.8)
    raise WrongScreen("스크롤 후 목록도 상세도 아님")


def load_json(path: Path, default):
    return json.loads(path.read_text()) if path.exists() else default


def save_json(path: Path, data):
    LOCAL.mkdir(exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2))
