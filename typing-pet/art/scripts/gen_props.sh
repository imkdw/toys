#!/usr/bin/env bash
# 책상 소품을 한 개씩 크게 그린다 (codex 이미지 생성). 책상에 그려 넣고 차이를 잘라 내던 방식보다 윤곽이 깨끗하다.
#   art/scripts/gen_props.sh [-n 후보수] [-j 동시실행수] [--force] [소품 id ...]   (id 를 안 주면 전부)
# 프롬프트: art/_props/sprites.json (props.<id> + tail). 결과: art/_props/sprite/<id>_<n>.png (마젠타 배경 1024x1024)
# art/_props/ref/<id>.* (실물 사진)이 있으면 같이 첨부해서 그 물건을 따라 그리게 한다.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SPEC="$ROOT/art/_props/sprites.json"
OUT="$ROOT/art/_props/sprite"
PY="$ROOT/art/.venv/bin/python"
cands=2; jobs=6; force=0; ids=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -n) cands="$2"; shift 2 ;;
    -j) jobs="$2"; shift 2 ;;
    --force) force=1; shift ;;
    *) ids+=("$1"); shift ;;
  esac
done
mkdir -p "$OUT/logs" "$OUT/work"
if [[ ${#ids[@]} -eq 0 ]]; then
  while IFS= read -r l; do ids+=("$l"); done < <("$PY" -c 'import json,sys; print("\n".join(json.load(open(sys.argv[1]))["props"]))' "$SPEC")
fi
# 선 굵기/색/시점 참고: 빈 책상
REF="$ROOT/art/_desk/desk_empty.png"
run_one() {
  local id="$1" n="$2" name="$1_$2"
  local out="$OUT/$name.png" work="$OUT/work/$name" log="$OUT/logs/$name.log"
  if [[ -f "$out" && "$FORCE" != 1 ]]; then echo "SKIP $name"; return 0; fi
  rm -rf "$work"; mkdir -p "$work"; cp "$REF" "$work/ref.png"
  # 실물 사진이 있으면 item.png 로 같이 첨부
  local imgs=(ref.png) item_note="" item
  item="$(ls "$ROOT/art/_props/ref/$id".* 2>/dev/null | head -1 || true)"
  if [[ -n "$item" ]]; then
    "$PY" -c 'import sys; from PIL import Image; Image.open(sys.argv[1]).convert("RGB").save(sys.argv[2])' "$item" "$work/item.png"
    imgs+=(item.png)
    item_note="item.png is a photo of the real object: redraw THIS object (same shape, colours, label layout and label wording) simplified into the style above."
  fi
  local prompt
  prompt="$("$PY" -c 'import json,sys; s=json.load(open(sys.argv[1])); print("Draw one prop: " + s["props"][sys.argv[2]] + ".\n" + s["tail"] + "\n" + sys.argv[3])' "$SPEC" "$id" "$item_note")"
  ( cd "$work" && codex exec --skip-git-repo-check -s workspace-write "$prompt" -i "${imgs[@]}" </dev/null ) >"$log" 2>&1 || true
  local src="$work/out.png"
  if [[ ! -f "$src" ]]; then
    src="$(grep -oE '/[^ ]*/\.codex/generated_images/[^ ]+\.png' "$log" | tail -1 || true)"
  fi
  if [[ -n "$src" && -f "$src" ]]; then
    "$PY" -c 'import sys; from PIL import Image; im=Image.open(sys.argv[1]).convert("RGB"); im=im if im.size==(1024,1024) else im.resize((1024,1024), Image.LANCZOS); im.save(sys.argv[2])' "$src" "$out"
    echo "OK   $name"
  else
    echo "FAIL $name (로그: $log)"; return 1
  fi
}
export -f run_one
export ROOT OUT SPEC PY REF FORCE="$force"
list=()
for id in "${ids[@]}"; do for i in $(seq 1 "$cands"); do list+=("$id" "$i"); done; done
set +e
printf '%s\n' "${list[@]}" | xargs -n 2 -P "$jobs" bash -c 'run_one "$0" "$1"' | tee "$OUT/logs/_last_run.txt"
set -e
echo "성공 $(grep -c '^OK' "$OUT/logs/_last_run.txt" || true) / 실패 $(grep -c '^FAIL' "$OUT/logs/_last_run.txt" || true)"
