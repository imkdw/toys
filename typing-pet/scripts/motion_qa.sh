#!/usr/bin/env bash
# 움직임 품질 자동 판정 (docs/MOTION_QA.md). 결과: target/motion_qa/report.md
#   scripts/motion_qa.sh                전 캐릭터 (_ 로 시작하지 않는 것)
#   scripts/motion_qa.sh usagi rakko    고른 캐릭터만
# 환경 변수: SIZE (렌더 px, 기본 512), FPS (기본 15)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="target/motion_qa"
SIZE="${SIZE:-512}"
FPS="${FPS:-15}"
PY="art/.venv/bin/python"
[[ -x "$PY" ]] || PY="python3"
cargo build -q --release --bin typing-pet
APP="target/release/typing-pet"
chars=("$@")
if [[ ${#chars[@]} -eq 0 ]]; then
  for d in characters/*/; do
    c="$(basename "$d")"
    [[ "$c" == _* || ! -f "$d/manifest.toml" ]] && continue
    chars+=("$c")
  done
fi
render() { # 캐릭터 하나의 behavior 전부와 시나리오들
  local c="$1" d="$OUT/$1"
  rm -rf "$d"
  mkdir -p "$d"
  "$APP" --character-dir "characters/$c" --render-behaviors "$d/bhv" --pairs --fps "$FPS" --size "$SIZE" >/dev/null 2>>"$d/render.log"
  for sc in "mouse 6" "demo 17" "levels 14" "burst 13" "keys 15"; do
    set -- $sc
    "$APP" --character-dir "characters/$c" --render-timeline "$d/tl_$1" --scenario "$1" --secs "$2" --pairs --fps "$FPS" --size "$SIZE" >/dev/null 2>>"$d/render.log"
  done
}
for c in "${chars[@]}"; do render "$c" & done
wait
"$PY" art/scripts/motion_qa.py --out "$OUT" --size "$SIZE" "${chars[@]}"
