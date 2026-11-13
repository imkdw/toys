# 치이카와 행동 시트

## 성격 한 줄

겁 많고 잘 울지만 열심히 한다. 잘 울먹이고 부들부들 떤다.

## 대표 동작

근거 크롭은 `art/chiikawa/ref/behavior/`.

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 울먹이며 깜짝 (`wat`) | 26화 e026-011 (겁먹고 울먹이는 얼굴) | `wake` | overlay, head/overheat(울먹임 e185-028) + 땀방울 fall + 작은 tremble. 팔 번쩍 드는 full/wat_a 는 애니에 없어 쓰지 않음 | fx/sweat |
| 부들부들 떨기 | 224화 e224-020 | `level:dizzy` hold | overlay, 몸 tremble (작고 빠르게) | 없음 |
| 눈물 글썽 -> 주르륵 | 185화 e185-028 | `level:overheat` enter | overlay, 눈물 fall | head/overheat 를 울먹임으로 다시 그림, fx/tear |
| 엉엉 울기 | 224화, 185화 | `gesture` | full, `full/cry_a` 1장 + 눈물 fall + tremble (>< 눈 cry_b 는 애니에 없어 뺌) | fx/tear |
| 브로콜리 먹고 감동 | 4화 e004-006 | `idle` | full, `full/broccoli_a` 1장 + 숨쉬기 (>< 눈으로 씹는 broccoli_b 는 애니에 없어 뺌) | 기존 |
| 합격증 들어 보이기 | 60화 e060-026 | `milestone` | full, `full/license_a` 1장 (머리 위로 드는 license_b 는 애니에 없어 뺌) | 기존 |
| 브로콜리 누르면 감동 (`peek`) | 4화 e004-006 | `prop` | overlay, head/overheat(울먹임) + 눈물 fall | 없음 |

## 공통 머리 단계

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 기존 | 초점 없는 멍한 눈 + 땀방울 (안경 없음) | 화난 얼굴이 아니라 울먹임 (e185-028) | 기존 |

## 성격 타이밍

| 값 | 기본 | 치이카와 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 1.02 | 반응이 작다 |
| `_timing.paw_down_ms` | 120 | 90 | 조심조심 짧게 누른다 |
| `_anim.glow_ms` | 220 | 160 | 같은 이유 |
| `_timing.wake_after_s` | 15 | 10 | 잘 놀란다 |
