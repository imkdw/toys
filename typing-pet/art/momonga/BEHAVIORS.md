# 모몽가 행동 시트

근거 프레임은 `e<회차>@<분:초>` (회차 안 시각, `~/toys/video-binge-maker/work/final/final-with-ja.mp4` + chapters.json).
크롭은 `art/momonga/ref/real/` (생김새), `ref/behavior/` (동작/표정), `ref/scene/` (장면).

## 성격 한 줄

관심받고 싶어서 귀여운 척, 우는 척, 떼쓰기를 번갈아 하는 응석꾸러기. 남의 것을 슬쩍 가져가고 뻔뻔하게 웃는다.

## 생김새 (근거)

| 특징 | 근거 |
|---|---|
| 둥근 흰 몸, 양옆 털 뭉치 선, 둥근 귀 안쪽 하늘색 | e157@0:04, e132@0:00 (`ref/real/e157_0004_front.png`, `e132_0000_standing.png`) |
| 하늘색 동그란 코, 큰 반짝이는 눈, 빗금 볼터치 | e029@0:17, e118@0:01 (`ref/real/e029_0017_face.png`, `e118_0001_face.png`) |
| 몸만 한 하늘색 꼬리 (가장자리 빗금 털), 화면 오른쪽 뒤로 솟음 | e157@0:04, e132@0:00 |

## 대표 동작

| 동작 | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 떡(모찌) 먹고 황홀 | e068@0:02~0:06 (`ref/scene/mochi_e068_0003.png`) | `idle` | full `mochimochi`, 2장 교대 + 숨쉬기 | full/mochimochi_a, _b |
| "봐~ 봐~" 춤 | e157@0:08~0:09 (`ref/scene/bwa_e157_0008.png`) | `milestone` | full `bwabwa`, 2장 교대 + "봐~" 말풍선 pop_in | full/bwabwa_a, _b, fx/bwa |
| 떼쓰며 울기 | e029@0:20~0:22 (`ref/scene/tantrum_e029_0020.png`) | `gesture` | overlay `dadakko`, head/overheat(우는 척 e132@0:13) + tremble + 눈물 fall. >< 눈으로 우는 full/dadakko_a, _b 는 애니에 없어 쓰지 않음 | fx/droplet |
| 귀여운 척 반짝 눈 | e029@0:17~0:19 (`ref/behavior/excited_e029_0018.png`) | `level:excited` | overlay `kirakira`, head/excited + 몸 sway + 반짝이 pulse | head/excited, fx/sparkle |
| 우는 척 (거짓 울음) | e132@0:04, e132@0:12~0:17 (`ref/behavior/fakecry_e132_0013.png`) | `level:overheat` | overlay `usonaki`, head/overheat + 몸 tremble + 눈물 fall | head/overheat, fx/droplet |
| 바닥에서 버둥버둥 | e029@0:20~0:22 | `level:dizzy` hold | overlay `jitabata`, 몸 tremble 크게 + 머리 sway | 없음 |
| 뻔뻔한 곁눈질 | e132@0:05, e118@0:01 (`ref/behavior/smug_e132_0005.png`) | `wake` | overlay `smirk`, head/smug + 머리 nod | head/smug |
| 지우면 뾰로통 | e066@0:53 (`ref/behavior/pout_e066_0053.png`) | `key:backspace` | overlay `hmph`, head/pout + 몸 squash | head/pout |
| 봐 달라고 반짝 | e157@0:04~0:06 (`ref/real/e157_0004_front.png`) | `ambient` | overlay `lookatme`, head/excited + 몸 sway + 반짝이 | head/excited, fx/sparkle |
| 남의 것 슬쩍 | e066@0:30~0:33 (우사기 밥솥 뚜껑을 열어 먹음, `ref/behavior/snatch_e066_0031.png`), 표정 e132@0:05 | `prop` | overlay `snatch`, head/smug | head/smug |

## 허용 표정 (`_expressions`)

애니에 나온 표정만 그린다. 여기 없는 머리는 엔진이 그리지 않는다 (head/normal 은 항상 허용).

| 머리 | 근거 |
|---|---|
| head/excited | e029@0:18 귀여운 척: 크고 촉촉한 눈에 반짝이 하이라이트, 입 ω |
| head/overheat | e132@0:13 우는 척: 물결 모양으로 찡그린 눈 + 아래 눈물 맺힘/주르륵 |
| head/smug | e132@0:05 뻔뻔한 반쯤 감은 눈 + 입꼬리 올라간 ω |
| head/pout | e066@0:53 반쯤 감은 눈 + 미간 주름 + 입 삐죽 (밥 내놔) |

## 공통 머리 단계

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 귀여운 척 반짝 눈 (e029@0:18) | 근거 표정 없음: head/normal + `jitabata` 버둥 | 우는 척 (e132@0:13) | 근거 표정 없음: head/normal |

## 성격 타이밍

| 값 | 기본 | 모몽가 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 1.06 | 관심받으려고 반응을 크게 한다 |
| `_timing.level_up` | [12, 22, 32] | [10, 18, 28] | 금방 들뜨고 금방 운다 |
| `_anim.glow_ms` / `_timing.paw_down_ms` | 220 / 120 | 220 / 120 | 기본 |
| `_timing.wake_after_s` | 15 | 12 | 혼자 두면 금방 심심해한다 |
| `_timing.burst_keys` | 9 | 9 | 기본 |
| `_timing.ambient_every_s` | [60, 180] | [60, 140] | "날 봐" 를 자주 한다 (최소 60초는 지킴) |

## 책상 소품

**하얀 떡 두 알 (잎사귀 접시)**

- 근거: e068@0:00~0:01, e068@0:19~0:23 ("달콤한 디저트"). 모몽가가 풀밭에 놓인 잎사귀 위 떡 두 알을 먹는다 (`ref/behavior/desk_dango_e068_0020.png`, `ref/scene/mochi_e068_0000.png`). idle 장면 `mochimochi` 와 같은 떡.
- 그리는 법: 납작한 연두색(#9ED67A 정도) 잎사귀 한 장이 책상 위에 눕혀 있고 (가운데 잎맥 한 줄, 진갈색 얇은 외곽선), 그 위에 하얀 둥근 떡 두 알이 나란히 붙어 앉아 있다. 떡은 찹쌀떡처럼 아래가 살짝 납작한 둥근 덩어리, 순백색, 얇은 진갈색 외곽선, 표면에 작은 가루 무늬 선 2~3개. 전체 크기는 컵 하나 정도 (잎사귀 폭이 떡 두 알보다 조금 넓음), 정면에서 약간 위로 본 각도. 그림자/그라데이션 없음.

## 진행 상황

- 생성 완료/선택: base_2 (책상이 `art/_desk/usagi_noitems.png` 와 거의 같음), left_down_1, right_down_1, head_smug_1, scene_bwa_a_2.
- codex 사용량 한도로 나머지는 못 만듦. `art/momonga/TODO_regen.sh 1 -> 2 -> 3` 순서로 이어서 만든다. 예정 picks 는 `picks.json` 의 `_pending`.
- fx anchor 값(말풍선 [200,190], 반짝이 [250,250], 눈물 [650,540])은 base_2 기준 어림값. extract_fx 경계 상자를 보고 고친다.
- 머리 마스크가 얼굴 차이로만 잡히면 큰 꼬리가 몸 마스크 밖에 남는다. hop/sway 시트에서 꼬리가 끊겨 보이면 `masks/head.png` 에 꼬리를 더한다.

## 확인

- `typing-pet --check characters/momonga` 통과
- `typing-pet --character-dir characters/momonga --render-all <폴더>` 의 `bhv-*.png`
