#!/usr/bin/env bash
# codex 이미지 생성을 병렬로 돌리는 실행기 (PLAN 4.2.1).
#
# 사용법:
#   art/scripts/gen.sh [-c 캐릭터] -i <입력 이미지> [-r 참고 이미지]... [-p 동작 참고]... [-n 후보수] [-j 동시실행수] [--force] <프롬프트 파일...>
#
# - 캐릭터별 폴더 art/<캐릭터>/ (기본 usagi) 의 prompts/ 를 읽고 raw/ 에 쓴다.
#
# - 00_base.txt 는 _character.txt + _style.txt + 본문, 나머지는 _edit_head.txt + _character.txt + 본문으로 프롬프트를 만든다.
# - -r 로 실제 애니 캡처 같은 참고 이미지를 여러 장 함께 첨부한다 (입력 이미지 뒤에 붙음).
# - -p 로 따라 할 애니 장면(동작/표정/소품)을 첨부한다. 생김새 참고(-r)와 달리 자세를 따라 그리게 한다.
# - 출력 이름: 10_left_down.txt -> left_down, 2x_*.txt -> head_*, 후보가 여러 장이면 <이름>_1 ..
# - 작업마다 art/<캐릭터>/raw/work/<이름>/ 에서 codex 를 실행하고 결과를 art/<캐릭터>/raw/<이름>.png 로 옮긴다.
# - 로그는 art/<캐릭터>/raw/logs/<이름>.log. 출력 파일이 이미 있으면 건너뛴다 (--force 로 덮어쓰기).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
input=""
cands=1
jobs=6
force=0
char="usagi"
files=()
refs=()
poses=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -c) char="$2"; shift 2 ;;
    -i) input="$2"; shift 2 ;;
    -r) refs+=("$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"); shift 2 ;;
    -p) poses+=("$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"); shift 2 ;;
    -n) cands="$2"; shift 2 ;;
    -j) jobs="$2"; shift 2 ;;
    --force) force=1; shift ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) files+=("$1"); shift ;;
  esac
done
PROMPTS="$ROOT/art/$char/prompts"
RAW="$ROOT/art/$char/raw"
[[ -d "$PROMPTS" ]] || { echo "프롬프트 폴더 없음: $PROMPTS" >&2; exit 2; }
[[ -n "$input" && -f "$input" ]] || { echo "입력 이미지(-i)가 필요함" >&2; exit 2; }
[[ ${#files[@]} -gt 0 ]] || { echo "프롬프트 파일이 필요함" >&2; exit 2; }
input="$(cd "$(dirname "$input")" && pwd)/$(basename "$input")"
mkdir -p "$RAW/logs" "$RAW/work"

out_name() {
  local base num rest
  base="$(basename "$1" .txt)"
  num="${base%%_*}"
  rest="${base#*_}"
  if [[ "$num" == 2* ]]; then echo "head_$rest"; else echo "$rest"; fi
}

# 한 작업: 이름, 프롬프트 파일
run_one() {
  local name="$1" pfile="$2"
  local out="$RAW/$name.png" work="$RAW/work/$name" log="$RAW/logs/$name.log"
  if [[ -f "$out" && "$FORCE" != 1 ]]; then echo "SKIP $name"; return 0; fi
  rm -rf "$work"; mkdir -p "$work"
  cp "$INPUT" "$work/input.png"
  local imgs=(input.png) i=0
  for r in $REFS; do i=$((i + 1)); cp "$r" "$work/ref_$i.png"; imgs+=("ref_$i.png"); done
  i=0
  for r in $POSES; do i=$((i + 1)); cp "$r" "$work/pose_$i.png"; imgs+=("pose_$i.png"); done
  local pose_note=""
  if [[ -n "$POSES" ]]; then pose_note=" pose_*.png are real anime frames of the scene to act out: copy the action, expression and props from them, redrawn at the desk in the input image (not their background, framing or camera zoom)."; fi
  local prompt
  if [[ "$(basename "$pfile")" == 00_* ]]; then
    prompt="$(cat "$PROMPTS/_character.txt" "$PROMPTS/_style.txt" "$pfile")"
  else
    prompt="$(cat "$PROMPTS/_edit_head.txt" "$pfile"; echo; echo 'Character reference (do not change the design):'; cat "$PROMPTS/_character.txt")"
  fi
  prompt="$prompt
The attached images are: ${imgs[*]} (input.png first; ref_*.png are real anime screenshots of the character for the character design only, do not copy their backgrounds or poses).$pose_note
Generate exactly one image with your image generation tool.
Then save the generated image as ./out.png in the current directory, 1024x1024 PNG.
Do not create any other files and do not edit input.png."
  ( cd "$work" && codex exec --skip-git-repo-check -s workspace-write "$prompt" -i "${imgs[@]}" </dev/null ) >"$log" 2>&1 || true
  local src="$work/out.png"
  if [[ ! -f "$src" ]]; then
    # codex 가 지정 경로에 저장하지 않았으면 로그에 찍힌 생성 원본 경로를 쓴다
    src="$(grep -oE '/[^ ]*/\.codex/generated_images/[^ ]+\.png' "$log" | tail -1 || true)"
  fi
  if [[ -n "$src" && -f "$src" ]]; then
    "$ROOT/art/.venv/bin/python" - "$src" "$out" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
if im.size != (1024, 1024):
    im = im.resize((1024, 1024), Image.LANCZOS)
im.save(sys.argv[2])
PY
    echo "OK   $name"
  else
    echo "FAIL $name (로그: $log)"
    return 1
  fi
}
export -f run_one
REFS="${refs[*]:-}"
POSES="${poses[*]:-}"
export RAW PROMPTS FORCE="$force" INPUT="$input" ROOT REFS POSES

list=()
for f in "${files[@]}"; do
  [[ -f "$f" ]] || f="$PROMPTS/$f"
  [[ -f "$f" ]] || { echo "프롬프트 없음: $f" >&2; exit 2; }
  f="$(cd "$(dirname "$f")" && pwd)/$(basename "$f")"
  n="$(out_name "$f")"
  if [[ "$cands" -gt 1 ]]; then
    for i in $(seq 1 "$cands"); do list+=("${n}_$i" "$f"); done
  else
    list+=("$n" "$f")
  fi
done

set +e
printf '%s\n' "${list[@]}" | xargs -n 2 -P "$jobs" bash -c 'run_one "$0" "$1"' | tee "$RAW/logs/_last_run.txt"
set -e
ok=$(grep -c '^OK' "$RAW/logs/_last_run.txt" || true)
fail=$(grep -c '^FAIL' "$RAW/logs/_last_run.txt" || true)
echo "성공 $ok / 실패 $fail"
[[ "$fail" == 0 ]]
