# <캐릭터 이름> 행동 시트

새 캐릭터 작업의 첫 단계. 이 표가 그대로 `picks.json` 의 `_behaviors` 와 프롬프트 `6x_behavior_<이름>.txt` 목록이 된다.
cue, motion, fx 종류와 필드는 `characters/README.md` 의 "캐릭터 고유 행동" 참고.

## 성격 한 줄

(예: 겁 많고 잘 울지만 열심히 한다)

## 대표 동작 (최소 6개)

참고 회차/프레임 id 가 없는 동작은 넣지 않는다. 근거 크롭은 `art/<이름>/ref/behavior/<동작>.png`.

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| | | idle | | |
| | | milestone | | |
| | | gesture | | |
| | | wake / burst / level:* / egg / ambient / key:* | | |
| | | | | |
| | | | | |

- `idle`, `milestone`, `gesture` 에 각각 1개 이상, 그 밖의 cue 에 3개 이상.
- 절반 이상은 motion 이나 움직이는 fx 를 쓴다 (2장 교대만으로 끝내지 않는다).
- behavior 이름은 다른 캐릭터와 겹치지 않게 짓는다.

## 공통 머리 단계를 이 캐릭터는 어떻게 보여주나

머리 단계 그림(`head/excited` 등)과 단계 진입 반응(`level:*`)을 캐릭터 성격에 맞게 정한다.

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| | | | |

## 성격 타이밍

`picks.json` 의 `_timing` / `_anim` 으로 넣는다.

| 값 | 기본 | 이 캐릭터 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` (단계 오를 때 팝) | 1.04 | | |
| `_timing.level_up` (단계 오르는 점수) | [12, 22, 32] | | |
| `_anim.glow_ms` / `_timing.paw_down_ms` (앞발 누름) | 220 / 120 | | |
| `_timing.wake_after_s` | 15 | | |
| `_timing.burst_keys` | 9 | | |
| `_timing.ambient_every_s` (최소 60) | [60, 180] | | |

## 확인

- `typing-pet --check characters/<이름>` 통과
- `typing-pet --character-dir characters/<이름> --render-all <폴더>` 의 `bhv-*.png`
- `typing-pet --character-dir characters/<이름> --render-timeline <폴더> --scenario <wake|burst|levels|keys>`
