# 쿠리만쥬 행동 시트

## 성격 한 줄

느긋한 아저씨 같은 술꾼. 안주를 집어 먹고, 한 모금 들이켠 뒤 "하아~" 하고 숨을 내쉬는 게 낙이다. 반응은 크지 않고 묵직하게 천천히.

## 대표 동작

근거 크롭은 `art/kurimanju/ref/behavior/` (생김새는 `ref/real/`, 장면 따라 그리기용 프레임은 `ref/scene/`).
프레임 id 는 `e<회차>@<회차 안 시각 mm:ss>` (원본: `~/toys/video-binge-maker/work/final/final-with-ja.mp4`).

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 안주 집어 먹기 (`kuri_snack`) | e243@00:13~00:16 (술집에서 젓가락으로 안주), e242@00:06~00:08 (어묵 집어 먹기) | `idle` | full, 2장 교대 + 숨쉬기 | full/kuri_snack_a, _b |
| 생맥주 한잔 쭉 -> "하아~" (`kuri_cheers`) | e244@00:22~00:24 (조끼잔 들이켜고 하아), e243@00:21~00:22 (조끼잔), e180@00:07~00:10 (하이볼) | `milestone` | full, 맥주(황금색 + 흰 거품) 마시기 -> 잔을 내리고 하아 단계 크로스페이드 + 입김 float | full/kuri_drink_a, full/kuri_drink_b, fx/kuri_steam |
| "합격" 종이 들어 보이기 (`kuri_pass`) | e244@00:20~00:21 (술 자격증 합격 회상, 집중선) | `gesture` | full 1장 | full/kuri_pass_a |
| "저기요" 손 들기 (`kuri_order`) | e243@00:35~00:37 (가게에서 한 손 들어 주문) | `wake` | full 1장 + 몸 nod | full/kuri_order_a |
| "하아~" 내쉬기 (`kuri_haa`) | e35@00:08~00:09, e180@00:09~00:10, e242@00:28~00:29, e244@00:23~00:24 | `level:excited` | overlay, head/excited(하아 얼굴) + 말풍선 pop_in + 입김 float + 몸 squash | head/excited, fx/kuri_haa, fx/kuri_steam |
| 볼에서 김 (`kuri_flush`) | e242@00:10~00:12 | `level:overheat` | overlay, 입김 float 2번 + 몸 squash 작게 | head/overheat(= 발그레), fx/kuri_steam |
| 입맛 다시기 (`kuri_lick`) | e244@00:09 (혀 내밀고 쩝) | `ambient` | overlay, head/lick + 머리 nod | head/lick |
| 맥주 누르면 음미 (`kuri_savor`) | e244@00:10~00:14 (ω 입으로 음미), e35@00:18~00:19 | `prop` | overlay, head/savor + 머리 nod + 입김 float | head/savor, fx/kuri_steam |

- 모든 behavior 이름은 `kuri_` 로 시작해서 다른 캐릭터와 겹치지 않는다 (우사기에 `haa` 가 있음).
- 말풍선 "하아~" 는 ハーッ 숨소리를 한국어로 옮긴 것.

## 공통 머리 단계

애니에 나온 표정만 그린다 (`_expressions`). 없는 표정은 head/normal 로 둔다.

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| "하아~" 입 크게 벌린 얼굴 (e244@00:23, e35@00:09) | 볼 발그레 + 김 (e242@00:11) | dizzy 와 같은 발그레 얼굴 (e242@00:11) | 없음 (정면에서 눈 감은 프레임을 찾지 못함, head/normal) |

추가 표정: head/lick (e244@00:09), head/savor (e244@00:12, e35@00:18). 선글라스, 소용돌이 눈, 화난 김 같은 지어낸 표정은 만들지 않는다.

## 성격 타이밍

| 값 | 기본 | 쿠리만쥬 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 1.03 | 묵직해서 크게 튀지 않는다 |
| `_timing.level_up` | [12, 22, 32] | [14, 25, 36] | 느긋해서 쉽게 달아오르지 않는다 |
| `_anim.glow_ms` / `_timing.paw_down_ms` | 220 / 120 | 260 / 150 | 굵은 팔로 꾹꾹 느리게 누른다 |
| `_timing.wake_after_s` | 15 | 25 | 웬만해선 안 놀란다 |
| `_timing.burst_keys` | 9 | 11 | 같은 이유 |
| `_timing.ambient_every_s` | [60, 180] | [90, 240] | 말수가 적다 |

## 책상 소품

**원컵 사케 (One Cup OZEKI 병)**

- 근거: 검수 피드백(2026-11-17)으로 맥주 조끼잔 대신 실제 제품 사진 `art/_props/ref/onecup_photo.jpeg` 을 따라 그린 원컵 사케 병. 프롬프트 `art/_props/prompts/95_prop_onecup.txt`, 고른 후보 `prop_onecup_1`.
- 그리는 법: 투명한 유리 병, 위에 반투명 흰 플라스틱 뚜껑, 진한 파란 라벨에 흰 글씨 "SAKE / One CUP / OZEKI". 다른 캐릭터와 같은 얇은 진갈색 윤곽, 평평한 색.
  (예전 소품 맥주 조끼잔은 `art/_props/prompts/retired/95_prop_beer.txt`, 그 전의 "술 컵" 병은 `retired/95_prop_sake_cup.txt`.)
- 크기: 컵 하나 크기 (높이가 키보드 세로 폭과 비슷), 책상 위에 똑바로 선다.
- 상호작용: 누르면 `prop` cue 로 `kuri_savor` (음미하는 ω 입 + 끄덕 + 입김).

## 확인

- `typing-pet --check characters/kurimanju` 통과
- `typing-pet --character-dir characters/kurimanju --render-all <폴더>` 의 `bhv-*.png`
