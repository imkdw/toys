# 하치와레 행동 시트

## 성격 한 줄

밝고 낙천적, 말하고 노래한다. 몸을 흔들며 흥얼거린다.

## 대표 동작

근거 크롭은 `art/hachiware/ref/behavior/`.

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| "어떻게든 되겠지!" | 10화 e010-005 | `level:overheat` enter | overlay, 머리 nod, 말풍선 pop_in | head/overheat 를 땀 흘리며 웃는 얼굴로, fx/nantoka |
| 흥얼거리기 | 112화 e112-015, 205화 e205-017 | `level:excited` hold | overlay, 음표 2개 float (위상 엇갈림) + 머리 sway | fx/note_a, fx/note_b |
| 혀 내밀고 웃기 | 169화 e169-007 | `ambient` | overlay, 머리 바꾸기 짧게 | head/tongue |
| 책 읽기 | 201화 e201-022 | `idle` | full, 2장 교대 + 숨쉬기 | 기존 |
| 사진 찍기 | 216화 e216-005, 37화 | `milestone` | full, 2장 교대 + 플래시 | fx/white (자동) |
| 기타 치며 노래 | 169화 e169-007 | `gesture` | full, 2장 교대 + 음표 float + 몸 sway | fx/note_a, fx/note_b |

## 공통 머리 단계

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 기존 | 초점 없는 멍한 눈 + 땀방울 (안경 없음) | 땀 흘리며 활짝 웃음 (낙천적) | 기존 |

## 성격 타이밍

| 값 | 기본 | 하치와레 | 이유 |
|---|---|---|---|
| `_timing.ambient_every_s` | [60, 180] | [60, 120] | 자주 혼자 신난다 |
