#!/usr/bin/env bash
# 쿠리만쥬: codex 사용량 한도(2026-11-13 01:52 KST 무렵 풀림) 때문에 못 만든 그림을 이어서 생성한다.
# gen.sh 는 이미 있는 출력은 건너뛰므로 그대로 다시 돌려도 된다. 저장소 루트에서 실행.
#
# 이미 있는 것: raw/base_1..4 (base_2 를 골라 base_final 로 복사), left_down_1/2 (2 고름), right_down_2,
#   behavior_steam_1/2 (둘 다 왼팔까지 바뀌어서, steam_1 의 김 부분만 base_final 초록 배경 위에 옮긴 behavior_steam_clean 을 씀).
# 다 만든 뒤:
#   1. 후보를 눈으로 비교해 picks.planned.json 의 _1 을 고른 번호로 바꾸고 picks.json 으로 복사
#   2. 2단계(아래 STAGE 2) 생성 (snack_b 는 고른 snack_a 에서 편집, haa_b 는 고른 haa_a 에서 편집)
#   3. fx 위치 확인: extract_fx.py 가 찍는 경계 상자로 picks 의 kuri_steam [830, 330], kuri_haa [820, 250] anchor 조정
#   4. art/scripts/build.sh --name kurimanju && cargo build && ./target/debug/typing-pet --check characters/kurimanju
#   5. ./target/debug/typing-pet --character-dir characters/kurimanju --render-all <폴더> 로 시트 확인
set -euo pipefail
cd "$(dirname "$0")/../.."

G="art/scripts/gen.sh -c kurimanju -i art/kurimanju/raw/base_final.png -r art/kurimanju/ref/real/front_desk_e243_0036.png -r art/kurimanju/ref/real/front_table_e244_0015.png"
B=art/kurimanju/ref/behavior
S=art/kurimanju/ref/scene

# STAGE 1 (base_final 에서 편집)
$G -n 2 -j 4 10_left_down.txt 11_right_down.txt 12_right_center.txt 13_mouse.txt 14_mouse_click.txt 70_base_clean.txt &
$G -n 2 -j 2 -p $B/haa_e244_0023.png -p $B/haa_e35_0009.png 20_haa.txt &
$G -n 2 -j 2 -p $B/flushed_e242_0011.png 21_flushed.txt &
$G -n 2 -j 2 -p $B/lick_e244_0009.png 22_lick.txt &
$G -n 2 -j 2 -p $B/savor_e244_0012.png -p $B/savor_e35_0018.png 23_savor.txt &
$G -n 2 -j 2 -p $S/snack_a.png -p $S/snack_oden.png 40_scene_snack_a.txt &
$G -n 2 -j 2 -p $S/haa_a.png 42_scene_haa_a.txt &
$G -n 2 -j 2 -p $S/pass_a.png 44_scene_pass_a.txt &
$G -n 2 -j 2 -p $S/order_a.png 46_scene_order_a.txt &
$G -n 2 -j 2 61_behavior_haa_bubble.txt &
wait

# STAGE 2 (고른 첫 프레임에서 편집; SNACK_A / HAA_A 를 고른 후보로 바꿔서 실행)
SNACK_A="${SNACK_A:-scene_snack_a_1}"
HAA_A="${HAA_A:-scene_haa_a_1}"
R="-r art/kurimanju/ref/real/front_desk_e243_0036.png -r art/kurimanju/ref/real/front_table_e244_0015.png"
art/scripts/gen.sh -c kurimanju -i "art/kurimanju/raw/$SNACK_A.png" $R -p $S/snack_b.png -n 2 -j 2 41_scene_snack_b.txt &
art/scripts/gen.sh -c kurimanju -i "art/kurimanju/raw/$HAA_A.png" $R -p $S/haa_b.png -p $B/haa_e242_0028.png -n 2 -j 2 43_scene_haa_b.txt &
wait
