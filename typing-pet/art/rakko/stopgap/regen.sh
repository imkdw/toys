#!/usr/bin/env bash
# codex 생성 한도가 풀린 뒤 실행: 임시(*_pil) 레이어 대신 쓸 진짜 생성 그림을 만든다.
# 끝나면 후보를 눈으로 고르고 picks.json 을 BEHAVIORS.md "남은 일" 대로 바꾼 뒤 art/scripts/build.sh --name rakko.
set -euo pipefail
cd "$(dirname "$0")/../../.."
B=art/rakko/raw/base_final.png
R="-r art/rakko/ref/real/e54_0043_table.png -r art/rakko/ref/real/e52_0017_face_calm.png"
S=art/rakko/ref/scene; H=art/rakko/ref/behavior
G="art/scripts/gen.sh -c rakko -i $B $R -n 2 -j 3"
$G 10_left_down.txt 11_right_down.txt 12_right_center.txt 13_mouse.txt 14_mouse_click.txt 70_base_clean.txt
$G -p $H/smile_e52_0005.png 20_excited.txt
$G -p $H/stern_e53_0016.png 23_overheat.txt
$G -p $S/parfait_spoon_e54_0050.png -p $S/parfait_bliss_e54_0053.png 40_scene_parfait_a.txt
$G -p $S/slash_e51_0017.png 42_scene_slash_a.txt
$G -p $S/cocoa_e115_0038.png 44_scene_cocoa_a.txt
$G -p $H/bolt_e117_0009.png 65_behavior_bolt.txt
$G -p $H/dream_e115_0044.png -p $S/parfait_bliss_e54_0053.png 66_behavior_dream.txt
$G 60_behavior_order_bubble.txt 61_behavior_joa_bubble.txt 62_behavior_hurry_bubble.txt 63_behavior_thanks_bubble.txt
# 둘째 프레임/김은 고른 첫 프레임을 -i 로 (예: scene_parfait_a_1):
#   art/scripts/gen.sh -c rakko -i art/rakko/raw/scene_parfait_a_1.png $R -p $S/parfait_bliss_e54_0053.png -n 2 41_scene_parfait_b.txt
#   art/scripts/gen.sh -c rakko -i art/rakko/raw/scene_slash_a_1.png $R -p $S/slash_e51_0017.png -n 2 43_scene_slash_b.txt
#   art/scripts/gen.sh -c rakko -i art/rakko/raw/scene_cocoa_a_1.png $R -p $S/cocoa_steam_e115_0040.png -n 2 45_scene_cocoa_b.txt 67_behavior_steam.txt
