"""체험단 목록을 여러 번 다시 불러오며 기준에 맞는 상품을 신청한다.

목록은 불러올 때마다 큰 풀에서 98개쯤을 무작위로 보여 준다. 그래서 탭을 다시 눌러 새로 불러오기를 반복하고,
새 상품이 거의 안 나올 때까지 돈다. 판정은 classify.py (상품명 키워드), local/overrides.json 이 있으면 그게 우선.
결과는 local/results.json 에 상품명별로 남고, 결과가 확정된 상품은 다시 열지 않는다.

사용: apply.py [--limit N] [--only 상품명일부] [--passes N]
"""
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from trial import (  # noqa: E402
    DETAIL_APPLY, LOCAL, RESULTS, WrongScreen, any_label, cards, d, ensure_list, load_json, nodes,
    on_detail, on_list, open_trial_tab, save_json, scroll_down, session_expired, to_top, wait_label,
)

from classify import classify  # noqa: E402

OVERRIDES = load_json(LOCAL / "overrides.json", {})
results = load_json(RESULTS, {})
limit = int(sys.argv[sys.argv.index("--limit") + 1]) if "--limit" in sys.argv else 9999
passes = int(sys.argv[sys.argv.index("--passes") + 1]) if "--passes" in sys.argv else 30
deep = "--deep" in sys.argv  # 패스마다 목록 끝(더보기 포함)까지 훑는다
only = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
FINAL = ("신청 완료", "이미 신청됨", "맞는 사이즈 없음", "제외", "질문", "신청 불가")
SIZE_LIKE = re.compile(r"(?<![A-Z])(XS|S|M|L|XL|2XL|XXL|3XL|FREE|F)(?![A-Z])|프리|(?<![0-9])[1-9]\d{1,2}(?![0-9.])", re.I)


def plan_for(name):
    if name in OVERRIDES:
        return OVERRIDES[name]
    mode, why, policy = classify(name)
    return {"mode": mode, "why": why, **policy}

SIZE_RE = re.compile(r"(?<![0-9A-Z])(XL|2XL|XXL|105|110)(?![0-9])", re.I)
FREE_RE = re.compile(r"FREE|프리", re.I)
SHOE_RE = re.compile(r"(?<![0-9])(2[7-9]\d|3[0-2]\d)(?![0-9])")


def log(*a):
    print(time.strftime("%H:%M:%S"), *a, flush=True)


def record(name, status, **extra):
    results[name] = {"status": status, **extra, "at": time.strftime("%Y-%m-%d %H:%M")}
    save_json(RESULTS, results)
    log("=>", status, extra or "", "|", name[:40])


def back_to_list():
    for _ in range(5):
        if on_list():
            return
        d.press("back")
        time.sleep(1.2)
    raise WrongScreen("목록으로 못 돌아옴")


def node_with(label, left=None):
    for lbl, b, a in nodes():
        if lbl == label and (left is None or b[0] == left):
            return b
    return None


def tap(b):
    d.click((b[0] + b[2]) // 2, (b[1] + b[3]) // 2)


def detail_state(timeout=8):
    end = time.time() + timeout
    while time.time() < end:
        ns = nodes()
        labels = [lbl for lbl, _, _ in ns]
        if any(lbl.startswith("이미 신청한 체험단") for lbl in labels):
            return "applied", ns
        for lbl, b, a in ns:
            if b[0] == 232 and b[1] >= 2600 and a.get("class", "").endswith("Button") and lbl:
                return ("open" if lbl == DETAIL_APPLY else "other:" + lbl), ns
        time.sleep(0.3)
    return "timeout", []


def size_rank(item):
    """작을수록 먼저 고른다: 남성용 우선, 그다음 큰 사이즈."""
    t = item[0]
    score = 0 if "남성" in t else 1
    for rank, pat in enumerate((r"(?<![A-Z0-9])L(?![A-Z])", r"FREE|프리", r"(?<![A-Z])M(?![A-Z])", r"(?<![A-Z])S(?![A-Z])")):
        if re.search(pat, t, re.I):
            return (score, rank)
    return (score, 9)


def option_nodes(ns, dropdown_top):
    """드롭다운 위쪽에 쌓이는 옵션 노드 (left 68)."""
    out = []
    for lbl, b, a in ns:
        if b[0] == 68 and b[2] == 1372 and lbl and b[1] < dropdown_top and lbl != "옵션을 선택해주세요":
            out.append((lbl, b))
    return sorted(out, key=lambda x: x[1][1])


def choose_option(name, p):
    """옵션 시트에서 옵션을 고른다. 고른 옵션 문자열 목록, 또는 사유 문자열(실패)."""
    picked = []
    for _ in range(3):
        dd = node_with("옵션을 선택해주세요")
        if dd is None:
            return picked
        tap(dd)
        time.sleep(1.0)
        opts = option_nodes(nodes(), dd[1])
        if not opts:
            return "옵션 목록을 못 읽음"
        texts = [t for t, _ in opts]
        cand = [(t, b) for t, b in opts if "품절" not in t and not any(w in t for w in p.get("avoid", []))]
        if p["mode"] == "size":
            fit = [(t, b) for t, b in cand if SIZE_RE.search(t) or SHOE_RE.search(t)
                   or (p.get("free_ok") and FREE_RE.search(t))]
            if not fit and p.get("free_ok") and not any(SIZE_LIKE.search(t) for t in texts):
                fit = cand  # 사이즈 구분이 없는 단일 사이즈 소품
            if not fit and p.get("fallback"):
                fit = sorted(cand, key=size_rank)  # 기준에 맞는 게 없으면 가장 큰 옵션
            if not fit:
                return "맞는 사이즈 없음: " + " | ".join(texts)
            cand = fit
        if not cand:
            return "고를 옵션 없음: " + " | ".join(texts)
        t, b = cand[0]
        tap(b)
        picked.append(t)
        time.sleep(1.0)
    return picked


def process(name, p):
    state, ns = detail_state()
    title = next((lbl[4:] for lbl, _, _ in ns if lbl.startswith("상품명:")), "")
    if title and title.strip() != name.strip():
        log("상세 상품명 불일치:", title[:40])
    if state == "applied":
        return record(name, "이미 신청됨")
    if state != "open":
        return record(name, "신청 불가", button=state)
    tap(node_with(DETAIL_APPLY, left=232))

    # 위시 담기 시트 또는 옵션 시트 또는 바로 신청 페이지
    end = time.time() + 8
    while time.time() < end:
        if any_label("위시에 담고 신청하기"):
            any_label("위시에 담고 신청하기").click()
            time.sleep(1.5)
            continue
        if node_with("옵션을 선택해주세요") or node_with(DETAIL_APPLY, left=64) or any_label("신청 완료하기"):
            break
        time.sleep(0.3)

    picked = []
    if node_with("옵션을 선택해주세요"):
        picked = choose_option(name, p)
        if isinstance(picked, str):
            back_to_list()
            status = "맞는 사이즈 없음" if picked.startswith("맞는 사이즈") else "확인 필요"
            return record(name, status, detail=picked)

    if not any_label("신청 완료하기"):
        b = None
        for _ in range(10):
            b = node_with(DETAIL_APPLY, left=64)
            if b:
                break
            time.sleep(0.3)
        if b is None:
            back_to_list()
            return record(name, "확인 필요", detail="옵션 시트 신청 버튼 없음", option=picked)
        tap(b)
    btn = wait_label("신청 완료하기", timeout=8)
    if btn is None:
        back_to_list()
        return record(name, "확인 필요", detail="신청 페이지 안 열림", option=picked)
    time.sleep(0.8)  # 위시 토스트(목록보기)가 하단을 덮고 있으면 첫 탭이 먹지 않는다
    done = None
    for _ in range(3):
        b = node_with("신청 완료하기")
        if b is None:
            break
        tap(b)
        done = wait_label("체험단 신청이 완료되었습니다.", timeout=6)
        if done is not None:
            break
    if done is None:
        done = wait_label("체험단 신청이 완료되었습니다.", timeout=6)
    if done is None:
        back_to_list()
        return record(name, "확인 필요", detail="완료 팝업 없음", option=picked)
    d(text="닫기", className="android.widget.Button").click()
    time.sleep(1.0)
    record(name, "신청 완료", option=picked)
    back_to_list()


def clip_tap(b):
    top, bottom = max(b[1], 520), min(b[3], 2450)  # 위쪽 탭 줄(최대 y 469)과 아래 맨 위로 버튼 피함
    if bottom - top < 120:
        return False
    d.click((b[0] + b[2]) // 2, (top + bottom) // 2)
    return True


def done(n):
    return str(results.get(n, {}).get("status", "")).startswith(FINAL)


def one_pass():
    """목록을 위에서 끝까지 한 번 훑는다. (새로 본 상품 수, 처리한 상품 수)"""
    global limit
    open_trial_tab()
    to_top()
    stale, blank, seen, new_names, handled, dead_more = 0, 0, set(), 0, 0, 0
    while limit > 0:
        vis = cards()
        new = [n for n, _ in vis if n not in seen]
        seen.update(new)
        for n in new:
            if n in results:
                continue
            new_names += 1
            p = plan_for(n)
            if p["mode"] == "skip":
                record(n, "제외", why=p.get("why", ""))
            elif p["mode"] == "ask":
                record(n, "질문", why=p.get("why", ""))
        todo = [(n, b) for n, b in vis
                if plan_for(n)["mode"] in ("apply", "size") and not done(n) and (only is None or only in n)]
        clicked = False
        for n, b in todo:
            if clip_tap(b):
                log("처리:", n[:50], plan_for(n))
                try:
                    process(n, plan_for(n))
                except Exception as e:  # noqa: BLE001
                    log("오류:", repr(e)[:200])
                    fails = results.get(n, {}).get("fails", 0) + 1
                    results[n] = {**results.get(n, {}), "fails": fails}
                    if fails >= 2:
                        record(n, "확인 필요", detail="오류 반복: " + repr(e)[:150])
                    for _ in range(5):
                        if on_list():
                            break
                        d.press("back")
                        time.sleep(1.2)
                    if not on_list():
                        open_trial_tab()
                limit -= 1
                handled += 1
                clicked = True
                break
        if clicked:
            continue
        more = any_label("더보기")
        if new:
            dead_more = 0
        if more is not None and (not new or deep) and dead_more < 3:  # 눌러도 안 늘어나는 더보기 무한 반복 방지
            dead_more += 1
            more.click()
            time.sleep(2)
            continue
        if not vis:
            blank += 1  # 맨 위 배너 구간
            if blank >= 8:
                break
        else:
            blank = 0
            stale = 0 if new or todo else stale + 1
            if stale >= (60 if deep else 3):
                break
        scroll_down()
    log(f"패스 끝: 목록 {len(seen)}개, 새 상품 {new_names}개, 처리 {handled}개")
    return new_names, handled


def main():
    quiet = 0
    for i in range(passes):
        log(f"=== 패스 {i + 1}")
        try:
            new_names, handled = one_pass()
        except WrongScreen as e:
            log("화면 오류, 다시 시작:", e)
            for _ in range(5):
                if d(description="체험단").exists or session_expired():
                    break
                d.press("back")
                time.sleep(1.2)
            new_names, handled = 99, 0
        quiet = quiet + 1 if new_names < 3 and handled == 0 else 0
        if quiet >= 3 or limit <= 0 or only:
            break
    counts = {}
    for r in results.values():
        counts[r.get("status", "?")] = counts.get(r.get("status", "?"), 0) + 1
    log("요약:", counts)


if __name__ == "__main__":
    main()
