#!/usr/bin/env bash
# public/pets/ 를 다시 만든다 (git 에 올리지 않는 생성물).
# ../typing-pet 앱의 오프스크린 렌더(실제 상태 머신/합성)를 800px 투명 프레임으로 뽑아 webp 로 바꾼다.
#   scripts/render-pets.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PET="$(cd "$ROOT/../typing-pet" && pwd)"
RAW="$ROOT/gen/raw"
PY="$PET/art/.venv/bin/python"
(cd "$PET" && cargo build -q --release --bin typing-pet)
APP="$PET/target/release/typing-pet"
rm -rf "$RAW" "$ROOT/public/pets"
mkdir -p "$RAW"

# 캐릭터별 책상 직군 (Friends, Idle 장면)
bhv() { "$APP" --character-dir "$PET/characters/$1" --job "$2" --render-behaviors "$RAW/bhv_$1" --fps 30 --size 800 >/dev/null; }
# 시나리오 타임라인: 이름, 캐릭터, 직군, 시나리오, 초, [--keys]
tl() { "$APP" --character-dir "$PET/characters/$2" --job "$3" --render-timeline "$RAW/$1" --scenario "$4" --secs "$5" --fps 30 --size 800 ${6:-} >/dev/null; }

bhv usagi basic & bhv chiikawa developer & bhv hachiware designer & bhv momonga marketer &
bhv kurimanju finance & bhv rakko security & bhv shisa infra & bhv kani planner &
tl tl_typing chiikawa developer demo 7 --keys &
# 설치 가이드의 "입력 키 보기" 장면: 같은 타이핑을 말풍선 없이
tl tl_typing_plain chiikawa developer demo 7 &
tl tl_levels hachiware designer levels 14 --keys &
tl tl_mouse momonga marketer mouse 6 --keys &
# Jobs 장면: 같은 burst 시나리오를 직군만 바꿔 뽑고, 직군마다 다음 몰아치기 구간을 잘라 쓴다
tl job_developer usagi developer burst 9 & tl job_designer usagi designer burst 9 & tl job_infra usagi infra burst 9 &
wait

# 클립 이름: [원본 폴더, 시작 프레임, 끝 프레임(생략하면 전부)]
SPEC='{
  "usagi_ura": ["bhv_usagi/ura"], "usagi_yaha": ["bhv_usagi/yaha"], "usagi_sleep": ["bhv_usagi/sleep"],
  "rakko_parfait": ["bhv_rakko/rakko_parfait"], "kuri_snack": ["bhv_kurimanju/kuri_snack"], "kani_read": ["bhv_kani/kani_read"],
  "chiikawa_teary": ["bhv_chiikawa/teary"], "hachiware_humming": ["bhv_hachiware/humming"], "momonga_kirakira": ["bhv_momonga/kirakira"],
  "kuri_haa": ["bhv_kurimanju/kuri_haa"], "rakko_joa": ["bhv_rakko/rakko_joa"], "shisa_beam": ["bhv_shisa/shisa_beam"], "kani_joy": ["bhv_kani/kani_joy"],
  "typing": ["tl_typing", 0, 180], "typing_plain": ["tl_typing_plain", 0, 180], "levels": ["tl_levels", 0, 210], "mouse": ["tl_mouse", 30, 180],
  "job_developer": ["job_developer", 9, 81], "job_designer": ["job_designer", 99, 171], "job_infra": ["job_infra", 189, 261]
}'
"$PY" "$ROOT/scripts/conv.py" "$RAW" "$ROOT/public/pets" "$SPEC"
rm -rf "$RAW"
