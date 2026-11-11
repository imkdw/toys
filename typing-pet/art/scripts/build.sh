#!/usr/bin/env bash
# 생성 원본(art/<이름>/raw) -> 캐릭터 레이어(characters/<이름>) 후처리 전체 실행 (PLAN 3.5).
#   chroma -> align -> colormatch -> automask(마스크 없을 때만) -> slice -> preview
# 사용법: art/scripts/build.sh [--remask] [--name usagi]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PY="$ROOT/art/.venv/bin/python"
S="$ROOT/art/scripts"
remask=""
name="usagi"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --remask) remask="--force"; shift ;;
    --name) name="$2"; shift 2 ;;
    *) echo "알 수 없는 인자: $1" >&2; exit 2 ;;
  esac
done
A="$ROOT/art/$name"
W="$A/work"

# 고른 파일만 처리
mapfile_picks=$("$PY" -c "import json; d=json.load(open('$A/picks.json')); print(' '.join(sorted({v for k,v in d.items() if not k.startswith('_')})))")
srcs=()
for p in $mapfile_picks; do srcs+=("$A/raw/$p.png"); done

rm -rf "$W/chroma" "$W/aligned" "$W/matched"
"$PY" "$S/chroma.py" "${srcs[@]}" -o "$W/chroma"
base="$("$PY" -c "import json; print(json.load(open('$A/picks.json'))['base'])")"
cp "$W/chroma/$base.png" "$W/base.png"
vars=()
for p in $mapfile_picks; do [[ "$p" != "$base" ]] && vars+=("$W/chroma/$p.png"); done
"$PY" "$S/align.py" --base "$W/base.png" "${vars[@]}" -o "$W/aligned"
"$PY" "$S/colormatch.py" --base "$W/base.png" "$W"/aligned/*.png -o "$W/matched"
"$PY" "$S/automask.py" --name "$name" $remask
"$PY" "$S/extract_fx.py" --name "$name"
"$PY" "$S/slice.py" --name "$name"
"$PY" "$S/preview.py" --name "$name" -o "$W/preview"
