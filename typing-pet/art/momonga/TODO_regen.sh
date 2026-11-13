#!/usr/bin/env bash
# 모몽가: codex 사용량 한도(2026-11-13 01:4x KST 해제)로 못 만든 그림을 이어서 만드는 명령 모음.
# 저장소 루트에서 실행: bash art/momonga/TODO_regen.sh [1|2|3]
#   1: base_2 에서 바로 편집하는 것 전부 (앞발, 머리, 장면 첫 프레임, 소품, base_clean)
#   2: 1 에서 고른 장면 첫 프레임에서 둘째 프레임 (아래 A_* 변수를 고른 후보로 바꾼 뒤)
#   3: 빌드와 확인
# 이미 있는 출력(raw/<이름>.png)은 gen.sh 가 건너뛴다.
# 동시에 다른 캐릭터 생성이 많이 돌면 재연결 오류가 잦으니 -j 를 작게 둔다.
set -euo pipefail
cd "$(dirname "$0")/../.."
R=art/momonga/ref
B=art/momonga/raw/base_2.png
RR=(-r $R/real/e157_0004_front.png -r $R/real/e029_0017_face.png)
g() { art/scripts/gen.sh -c momonga -i "$1" "${RR[@]}" "${@:2}"; }

step="${1:-1}"
if [[ "$step" == 1 ]]; then
  # 이미 있음: left_down_1/2, right_down_1, head_smug_1, scene_bwa_a_2
  g $B -n 2 -j 3 11_right_down.txt 12_right_center.txt 13_mouse.txt 14_mouse_click.txt 70_base_clean.txt 61_behavior_sparkle.txt 62_behavior_droplet.txt &
  g $B -p $R/behavior/excited_e029_0018.png -n 2 -j 2 20_excited.txt &
  g $B -p $R/behavior/fakecry_e132_0013.png -p $R/behavior/fakecry_e132_0004.png -n 2 -j 2 23_overheat.txt &
  g $B -p $R/behavior/pout_e066_0053.png -n 2 -j 2 27_pout.txt &
  g $B -p $R/scene/mochi_e068_0003.png -p $R/scene/mochi_e068_0000.png -n 2 -j 2 40_scene_mochi_a.txt &
  g $B -p $R/scene/bwa_e157_0008.png -n 2 -j 2 42_scene_bwa_a.txt &
  g $B -p $R/scene/tantrum_e029_0020.png -n 2 -j 2 44_scene_tantrum_a.txt &
  g $B -p $R/scene/bwa_e157_0009.png -n 2 -j 2 60_behavior_bwa_bubble.txt &
  wait
  # head_smug_2 는 실패했지만 head_smug_1 이 좋아서 다시 안 만든다.
fi

if [[ "$step" == 2 ]]; then
  # 1 의 후보를 눈으로 비교해 고른 첫 프레임 (picks.json 의 full/*_a 와 같게)
  A_MOCHI=art/momonga/raw/scene_mochi_a_1.png
  A_BWA=art/momonga/raw/scene_bwa_a_2.png
  A_TANTRUM=art/momonga/raw/scene_tantrum_a_1.png
  g $A_MOCHI -p $R/scene/mochi_e068_0003.png -n 2 -j 2 41_scene_mochi_b.txt &
  g $A_BWA -p $R/scene/bwa_e157_0009.png -n 2 -j 2 43_scene_bwa_b.txt &
  g $A_TANTRUM -p $R/scene/tantrum_e029_0020.png -n 2 -j 2 45_scene_tantrum_b.txt &
  wait
fi

if [[ "$step" == 3 ]]; then
  # picks.json: _pending 의 줄을 고른 후보 이름으로 바꿔 위(최상위)로 옮기고 _pending 을 지운 뒤 실행.
  # extract_fx.py 는 _fx 의 src 파일이 없으면 실패하므로 소품 3장(bwa, sparkle, droplet)이 있어야 한다.
  # 머리 마스크(automask)는 head/excited, head/overheat 의 차이로만 잡히므로 꼬리가 몸 마스크 밖에 남는다.
  # hop/sway 때 꼬리가 몸과 같이 움직이지 않으면 art/momonga/masks/head.png 에 꼬리 영역을 더해 다시 빌드한다.
  art/scripts/build.sh --name momonga
  cargo build
  ./target/debug/typing-pet --check characters/momonga
  out="${TMPDIR:-/tmp}/momonga-render"
  ./target/debug/typing-pet --character-dir characters/momonga --render-all "$out"
  echo "시트: $out (bhv-*.png 를 눈으로 확인하고 fx anchor 를 extract_fx 경계 상자에 맞춰 고친다)"
fi
