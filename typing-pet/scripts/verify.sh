#!/usr/bin/env bash
# 자동 검증 (PLAN 6.7). 결과는 target/verify/report.md.
#
#   scripts/verify.sh                 1층(cargo test) + 3층 오프스크린 조합 시트 + 기준 이미지 비교
#   scripts/verify.sh --live          위 + 2층 실제 입력(probe) + 실제 창 캡처 + 4층 부드러움/CPU/메모리
#   scripts/verify.sh --live --bundle     --live 를 .app 번들(디버그, 같은 인증서 서명) 실행 상태로
#   scripts/verify.sh --update-baseline   기준 이미지를 지금 렌더 결과로 갱신 (사람이 시트를 확인한 뒤에만)
#
# 환경 변수: CHARACTER(--live 에서 띄울 캐릭터. 기본: usagi 가 있으면 usagi, 없으면 _test), BURST_SECS(기본 180), IDLE_SECS(기본 30)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="target/verify"
PY="art/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"
live=0
bundle=0
update=""
for a in "$@"; do
  case "$a" in
    --live) live=1 ;;
    --bundle) bundle=1 ;;
    --update-baseline) update="--update" ;;
    *) echo "알 수 없는 인자: $a" >&2; exit 2 ;;
  esac
done
mkdir -p "$OUT"
REPORT="$OUT/report.md"
results=()
fail=0
record() { # 이름, 상태(PASS/FAIL/SKIP), 내용
  results+=("| $1 | $2 | $3 |")
  [[ "$2" == "FAIL" ]] && fail=1
  echo "[$2] $1: $3"
}

# ---------- 빌드 ----------
if ! cargo build -q --bins 2>"$OUT/build.log"; then
  record "빌드" FAIL "$OUT/build.log"
fi
APP="target/debug/typing-pet"
PROBE="target/debug/probe"

# ---------- 1층: 단위/무작위 테스트 ----------
if cargo test -q 2>&1 | tee "$OUT/test.log" | grep -q "FAILED\|panicked\|error\["; then
  record "1층 단위/무작위 테스트" FAIL "$OUT/test.log"
else
  n=$(grep -Eo "[0-9]+ passed" "$OUT/test.log" | awk '{s+=$1} END {print s}')
  record "1층 단위/무작위 테스트" PASS "${n}개 통과"
fi

# ---------- 캐릭터 고유 행동 규칙 (docs/CHARACTER_BEHAVIOR_PLAN.md 5.2) ----------
# _ 로 시작하지 않는 모든 캐릭터: 장면 cue 마다 behavior, 그 밖 cue 에 3개 이상, 절반 이상 움직임, 다른 캐릭터와 겹치지 않음
for d in characters/*/; do
  c="$(basename "$d")"
  [[ "$c" == _* || ! -f "$d/manifest.toml" ]] && continue
  if out=$("$APP" --check "characters/$c" 2>&1); then
    record "고유 행동 ($c)" PASS "$(echo "$out" | head -1)"
  else
    echo "$out" >"$OUT/check_$c.log"
    record "고유 행동 ($c)" FAIL "$(echo "$out" | grep -c '^  - ')개 규칙 위반: $OUT/check_$c.log"
  fi
done

# ---------- 3층: 오프스크린 조합 시트 + 기준 이미지 ----------
chars=("_test")
# 실제 캐릭터: characters/ 아래 manifest.toml 이 있고 _ 로 시작하지 않는 폴더 전부 (usagi 를 맨 뒤에 두어 --live 기본값으로)
for d in characters/*/; do
  c="$(basename "$d")"
  [[ "$c" == _* || "$c" == usagi || ! -f "$d/manifest.toml" ]] && continue
  chars+=("$c")
done
if [[ -f characters/usagi/manifest.toml ]]; then chars+=("usagi"); fi
for c in "${chars[@]}"; do
  dir="$OUT/combos/$c"
  rm -rf "$dir"
  if "$APP" --character-dir "characters/$c" --render-all "$dir" --size 240 >/dev/null 2>"$OUT/render_$c.log"; then
    "$PY" art/scripts/preview.py --character-dir "characters/$c" -o "$OUT/preview/$c" >/dev/null 2>&1 \
      && record "3층 조합 시트 ($c)" PASS "$OUT/preview/$c/sheet.png ($(ls "$dir" | wc -l | tr -d ' ')장), timeline.gif" \
      || record "3층 조합 시트 ($c)" FAIL "preview.py 실패"
    if [[ "$c" == "_test" ]]; then base="tests/baseline/_test"; else base="characters/$c/baseline"; fi
    if [[ ! -d "$base" && -z "$update" ]]; then
      record "3층 기준 이미지 ($c)" SKIP "기준 없음. 시트 확인 후 --update-baseline"
    elif msg=$("$PY" scripts/verify_tools.py compare "$dir" "$base" $update); then
      record "3층 기준 이미지 ($c)" PASS "$msg"
    else
      record "3층 기준 이미지 ($c)" FAIL "$msg"
    fi
  else
    record "3층 오프스크린 렌더 ($c)" FAIL "$OUT/render_$c.log"
  fi
done

# ---------- 2층 + 실제 창 + 4층 ----------
if [[ "$live" == 1 ]]; then
  CHAR="${CHARACTER:-${chars[${#chars[@]}-1]}}"
  CFG="$OUT/live_config.toml"
  TRACE="$OUT/trace.jsonl"
  printf 'character = "%s"\nposition = [1100.0, 600.0]\nsize = 240.0\nvisible = true\nbreathing = false\n' "$CHAR" >"$CFG"
  rm -f "$TRACE"
  if [[ "$bundle" == 1 ]]; then
    scripts/bundle.sh --debug >"$OUT/bundle.log" 2>&1 || record "번들 빌드" FAIL "$OUT/bundle.log"
    pkill -f "bundle-debug/TypingPet.app" 2>/dev/null
    rm -f "$HOME/Library/Logs/typing-pet.log"
    open -n --env TYPING_PET_CONFIG="$ROOT/$CFG" --env TYPING_PET_TRACE="$ROOT/$TRACE" target/bundle-debug/TypingPet.app
    for _ in $(seq 1 50); do APP_PID=$(pgrep -f "bundle-debug/TypingPet.app/Contents/MacOS" | head -1); [[ -n "$APP_PID" ]] && break; sleep 0.2; done
    for _ in $(seq 1 50); do [[ -f "$HOME/Library/Logs/typing-pet.log" ]] && grep -q "창 번호" "$HOME/Library/Logs/typing-pet.log" && break; sleep 0.2; done
    cp "$HOME/Library/Logs/typing-pet.log" "$OUT/app.log"
    sleep 1
    cp "$HOME/Library/Logs/typing-pet.log" "$OUT/app.log"
  else
    TYPING_PET_CONFIG="$CFG" TYPING_PET_TRACE="$TRACE" "$APP" >"$OUT/app.log" 2>&1 &
    APP_PID=$!
  fi
  trap 'kill $APP_PID 2>/dev/null' EXIT
  for _ in $(seq 1 50); do grep -q "창 번호" "$OUT/app.log" && break; sleep 0.2; done
  WID=$(grep -o "창 번호 Some([0-9]*)" "$OUT/app.log" | grep -o "[0-9]*")
  if ! grep -q "입력 감지 시작" "$OUT/app.log"; then
    record "2층 입력 감지" FAIL "CGEventTap 생성 실패 (입력 모니터링 권한): $OUT/app.log"
  else
    sleep 1
    # 실제 창 캡처: 가만히 있는 상태와 오프스크린 비교
    sleep 2
    screencapture -x -o -l "$WID" "$OUT/window_idle.png" 2>/dev/null
    px=$(sips -g pixelWidth "$OUT/window_idle.png" 2>/dev/null | awk '/pixelWidth/ {print $2}')
    "$APP" --character-dir "characters/$CHAR" --render-state '{"paw_l":"up","paw_r":"up","head":"normal","pose":null}' --out "$OUT/offscreen_idle.png" --size "${px:-240}" >/dev/null 2>&1
    if msg=$("$PY" scripts/verify_tools.py capture "$OUT/window_idle.png" "$OUT/offscreen_idle.png"); then
      record "3층 실제 창 캡처" PASS "$msg"
    else
      record "3층 실제 창 캡처" FAIL "$msg ($OUT/window_idle.png, 그 사이 마우스를 움직이면 다를 수 있음)"
    fi
    rss0=$(ps -o rss= -p $APP_PID | tr -d ' ')
    for p in all-keys modifiers repeat mouse burst; do
      "$PROBE" "$p" --trace "$TRACE" --out "$OUT/probe_$p.json" --burst-secs "${BURST_SECS:-180}" >"$OUT/probe_$p.log" 2>&1
      code=$?
      summary=$("$PY" -c "import json,sys; d=json.load(open('$OUT/probe_$p.json')); print(f\"보냄 {d['sent']} 받음 {d['received']} 실패 {len(d['failures'])} {'; '.join(d['failures'][:3])} {' / '.join(d['notes'])} 건너뜀 {d['skipped']}\")" 2>/dev/null || tail -2 "$OUT/probe_$p.log")
      case $code in
        0) record "2층 probe $p" PASS "$summary" ;;
        4) record "2층 probe $p" SKIP "손쉬운 사용 권한 없음 (통과로 치지 않음)"; fail=1 ;;
        *) record "2층 probe $p" FAIL "$summary" ;;
      esac
      sleep 1
    done
    rss1=$(ps -o rss= -p $APP_PID | tr -d ' ')
    # 4층: 부드러움 (트레이스의 프레임 간격, 입력->그리기 지연).
    # 기준 측정: 같은 창에서 지우기만 하며 연속으로 그려 시스템이 주는 프레임 간격을 잰다
    TYPING_PET_CONFIG="$CFG" TYPING_PET_NO_INPUT=1 TYPING_PET_BENCH=4 TYPING_PET_BENCH_LOOP=1 TYPING_PET_BENCH_EMPTY=1 \
      TYPING_PET_TRACE="$OUT/ref_frames.jsonl" "$APP" >/dev/null 2>&1
    if msg=$("$PY" scripts/verify_tools.py frames "$TRACE" "$OUT/ref_frames.jsonl"); then
      record "4층 프레임 간격/지연" PASS "$msg"
    else
      record "4층 프레임 간격/지연" FAIL "$msg"
    fi
    # 4층: 가만히 있을 때 CPU (숨쉬기 끔). 사용자 입력이 섞이지 않게 입력 감지를 끈 별도 인스턴스로 잰다
    TYPING_PET_CONFIG="$CFG" TYPING_PET_NO_INPUT=1 "$APP" >"$OUT/app_idle.log" 2>&1 &
    IDLE_PID=$!
    sleep 5
    : >"$OUT/cpu.log"
    ps -o time= -p $IDLE_PID >>"$OUT/cpu.log"
    for _ in $(seq 1 "${IDLE_SECS:-30}"); do sleep 1; done
    ps -o time= -p $IDLE_PID >>"$OUT/cpu.log"
    echo "${IDLE_SECS:-30}" >>"$OUT/cpu.log"
    kill $IDLE_PID 2>/dev/null
    if msg=$("$PY" scripts/verify_tools.py cpu "$OUT/cpu.log"); then
      record "4층 가만히 있을 때 CPU" PASS "$msg"
    else
      record "4층 가만히 있을 때 CPU" FAIL "$msg"
    fi
    # 4층: 메모리 (burst 전후 RSS)
    if [[ -n "$rss0" && -n "$rss1" ]]; then
      growth=$(( (rss1 - rss0) / 1024 ))
      if (( growth < 20 )); then
        record "4층 메모리" PASS "probe 전 ${rss0}KB -> 후 ${rss1}KB"
      else
        record "4층 메모리" FAIL "probe 동안 ${growth}MB 증가 (${rss0}KB -> ${rss1}KB)"
      fi
    fi
  fi
  kill $APP_PID 2>/dev/null
  wait $APP_PID 2>/dev/null
  trap - EXIT

  # 4단계: 저장된 위치 유지, 화면 밖 위치 보정 (입력 감지 끈 인스턴스로)
  swiftc -O scripts/window_bounds.swift -o "$OUT/window_bounds" 2>/dev/null
  pos_check() { # x y -> "x y w h mw mh"
    printf 'character = "%s"\nposition = [%s, %s]\nsize = 240.0\nvisible = true\nbreathing = false\n' "$CHAR" "$1" "$2" >"$OUT/pos_config.toml"
    TYPING_PET_CONFIG="$OUT/pos_config.toml" TYPING_PET_NO_INPUT=1 "$APP" >"$OUT/app_pos.log" 2>&1 &
    local pid=$!
    for _ in $(seq 1 50); do grep -q "창 번호" "$OUT/app_pos.log" && break; sleep 0.2; done
    sleep 0.5
    local wid
    wid=$(grep -o "창 번호 Some([0-9]*)" "$OUT/app_pos.log" | grep -o "[0-9]*")
    "$OUT/window_bounds" "$wid"
    kill $pid 2>/dev/null
    wait $pid 2>/dev/null
  }
  read -r x y w h mw mh <<<"$(pos_check 300 200)"
  if [[ "$x" == 300 && "$y" == 200 && "$w" == 240 ]]; then
    record "4단계 저장 위치 유지" PASS "저장 (300, 200) -> 실제 ($x, $y) ${w}x${h}"
  else
    record "4단계 저장 위치 유지" FAIL "저장 (300, 200) -> 실제 ($x, $y) ${w}x${h}"
  fi
  read -r x y w h mw mh <<<"$(pos_check -6000 -6000)"
  if (( x >= 0 && y >= 0 && x + w <= mw && y + h <= mh && x > mw / 2 && y > mh / 2 )); then
    record "4단계 화면 밖 위치 보정" PASS "저장 (-6000, -6000) -> 주 화면 우하단 ($x, $y), 화면 ${mw}x${mh}"
  else
    record "4단계 화면 밖 위치 보정" FAIL "저장 (-6000, -6000) -> ($x, $y), 화면 ${mw}x${mh}"
  fi
else
  record "2층 실제 입력 / 4층" SKIP "--live 로 실행"
fi

{
  echo "# typing-pet 검증 결과"
  echo
  echo "- 시각: $(date '+%Y-%m-%d %H:%M:%S')"
  echo "- 커밋: $(git rev-parse --short HEAD 2>/dev/null) $(git diff --quiet 2>/dev/null || echo '(작업 중 변경 있음)')"
  echo "- 모드: $([[ $live == 1 ]] && echo '--live' || echo '기본')"
  echo
  echo "| 항목 | 결과 | 내용 |"
  echo "|---|---|---|"
  printf '%s\n' "${results[@]}"
  echo
  echo "스크린샷/시트: $OUT/preview/*/sheet.png, $OUT/window_idle.png, 트레이스: $OUT/trace.jsonl"
} >"$REPORT"
echo "보고서: $REPORT"
exit $fail
