# 우사기 행동 시트

## 성격 한 줄

시끄럽고 예측 불가, 몸이 먼저 나간다. 소리 지르고 뛰고 떤다.

## 대표 동작

근거 크롭은 `art/usagi/ref/behavior/`. 프레임 id 는 `~/toys/video-binge-maker/work/usagi-gallery/` 의 사진 id.

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| "우라!" 외침 | 66화 e066-007, 15화 e015-014 | `level:excited` enter | overlay, 말풍선 pop_in (통통 뛰기는 뺌) | fx/ura (말풍선 "우라!") |
| 프루루루 고속 진동 | 134화 e134-023 | `level:dizzy` hold | overlay, 몸 tremble (빠르게) | 없음 |
| 과열에 부르르 (`fired`) | 134화 e134-023 (김 내며 떠는 장면) | `level:overheat` enter | overlay, 몸 tremble 2pt (애니에 없는 들썩/끄덕은 뺌) | 없음 |
| "하아?" | 36화 e036-024 | `wake` | overlay, 머리 바꾸기 + nod, 말풍선 pop_in | head/haa, fx/haa (말풍선 "하아?") |
| 귀 쫑긋 | 204화 e204-007 | `ambient` | overlay, 머리 sway 짧게 | 없음 |
| 졸기 | 만화 2023-02-06, 2026-09-09 | `idle` | full, 감은 눈 `full/sleep_a` + 숨쉬기 + 콧방울/Zzz (반쯤 뜬 눈 렘 프레임은 근거와 달라 뺌) | 기존 |
| 야하! | 204화 e204-016 | `milestone` | full, `full/yaha_a` 1장 + 말풍선 "야하!" (애니에 없는 양팔 벌린 yaha_b 와 통통은 뺌) | fx/yaha |
| 지팡이 휘두르기 | 36화 | `gesture` | full, 2장 교대 + 몸 sway | 기존 |

## 공통 머리 단계

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 입 크게 벌린 "야하" 얼굴 (기존) | 초점 없는 멍한 눈 + 땀방울 (안경/무지개 없음) | 찡그린 눈썹 + 볼에서 김 (e134-023, 빨간색 없음) | 귀 처짐 (기존) |

## 성격 타이밍

| 값 | 기본 | 우사기 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 1.08 | 단계가 오를 때 크게 반응 |
| `_timing.level_up` | [12, 22, 32] | [10, 19, 28] | 쉽게 흥분한다 |
| `_timing.burst_keys` | 9 | 8 | 조금만 몰아쳐도 burst |
