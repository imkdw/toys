# 시사 행동 시트

라멘 가게 "로우" 에서 일하는 시사. 근거는 실제 애니(한국어 더빙) 프레임이고 `e<회차>@<분:초>` 는 회차 안의 시각,
`pv2@<분:초>` 는 공식 2탄 PV (めざましテレビチャンネル "アニメ『ちいかわ』第2弾PV") 안의 시각이다.
말풍선 대사는 쓰지 않는다 (더빙 대사를 받아 적지 않았음).

## 성격 한 줄

성실하고 적극적인 노력가. 가게 일을 하면서 틈틈이 자격증 공부를 하고, 잘되면 온 얼굴로 기뻐한다.

## 생김새 근거 (`ref/real/`)

| 크롭 | 근거 | 본 것 |
|---|---|---|
| `e187_0046_standing.png` | e187@0:46 | 정면 전신. 크림색 둥근 몸, 양옆 주황 갈기 곱슬, 작은 귀 |
| `e250_0042_standing.png` | e250@0:42 | 정면 전신 (봉지 들고 서 있음) |
| `e123_0012_face_smile.png` | e123@0:12 | 얼굴 클로즈업. 눈 위 주황 소용돌이 눈썹, 빗금 볼터치 |
| `e165_0016_face.png` | e165@0:16 | 얼굴 클로즈업 (고구마빵 들고 있음) |
| `e124_0003_stump_front.png` | e124@0:03 | 그루터기에 앉은 전신 |

## 대표 동작

근거 크롭은 `ref/behavior/`, 장면 캡처는 `ref/scene/`. behavior 이름은 `shisa_` 로 시작한다.

| 동작 (behavior) | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 자격증 공부 (`shisa_study`) | e246@0:15 그루터기에서 공책에 연필로 쓰기, e251@0:39~0:45 | `idle` | full, 2단계 교대 + 숨쉬기 | full/shisa_study_a, _b |
| 합격! 자격증 들어 보이기 (`shisa_pass`) | e124@0:57 카드 들고 반짝, e124@0:30 문제집 들어 보임 | `milestone` | full, 2장 교대 + hop | full/shisa_pass_a, _b |
| 라멘 내기 (`shisa_ramen`) | e251@0:12 흰 두건 쓰고 라멘 그릇 들기, e166@0:12 | `gesture` | full, 2단계 교대 + 숨쉬기 | full/shisa_ramen_a, _b |
| 활짝 웃음 (`shisa_beam`) | e246@0:12 눈 감고 입 벌려 웃음 | `level:excited` | overlay, head/excited + 몸 hop | head/excited |
| 열 나게 공부 (`shisa_fever`) | e251@0:45 얼굴이 달아오르고 눈이 풀림 | `level:overheat` | overlay, head/overheat + 몸 sway 천천히 | head/overheat |
| 깜짝 (`shisa_startle`) | e250@0:45 멈춰 서서 굳음 (`ref/behavior/startle_e250_0045.png`) | `wake` | overlay, 몸 hop 1회 | 없음 |
| 흥얼흥얼 (`shisa_hum`) | pv2@0:48 캐릭터 소개 컷, 음표와 함께 몸 흔들기 | `ambient` | overlay, 음표 fx float + 몸 sway | fx/shisa_notes |
| 라멘 나왔습니다 (`shisa_serve`) | e251@0:12, e166@0:12 | `prop` | overlay, head/excited + 머리 nod | head/excited |

- `idle`, `milestone`, `gesture` 각 1개, 그 밖의 cue 5개.
- 8개 모두 motion, 움직이는 fx, 숨쉬기, 크로스페이드 중 하나 이상을 쓴다.

## 공통 머리 단계를 이 캐릭터는 어떻게 보여주나

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 눈 감고 입 벌려 웃음 (e246@0:12) | 없음 (근거 프레임 없음, head/normal) | 달아오른 얼굴에 풀린 눈 (e251@0:45) | 없음 (head/normal) |

선글라스, 소용돌이 눈 같은 지어낸 표정은 만들지 않는다.

## 성격 타이밍

| 값 | 기본 | 시사 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 기본 | 올리면 motion_qa T3 에 걸린다 |
| `_timing.level_up` | [12, 22, 32] | [12, 22, 34] | 꾸준히 버티다가 끝에 가서 열이 난다 |
| `_anim.glow_ms` / `_timing.paw_down_ms` | 220 / 120 | 220 / 120 | 기본 |
| `_timing.wake_after_s` | 15 | 15 | 기본 |
| `_timing.burst_keys` | 9 | 9 | 기본 |
| `_timing.ambient_every_s` | [60, 180] | [70, 180] | 가끔 흥얼거린다 |

## 책상 소품

**라멘 한 그릇 (`ramen`)**

- 근거: e251@0:12, e166@0:12 (가게에서 내는 라멘). 크롭 `ref/scene/ramen_e251_0012.png`, `ref/behavior/bowl_e166_0012.png`.
- 그리는 법: `art/_props/prompts/104_prop_ramen.txt`. 연한 민트색 그릇, 면과 고명, 김/젓가락/글자 없음.
- 반응: 누르면 `shisa_serve`.

## 지금 상태 (2026-11-18 끝남)

모든 그림이 생성본이다. 설치본은 `characters/shisa`.

| 항목 | 쓰는 그림 | 메모 |
|---|---|---|
| base, 앞발/마우스, base_clean | base_3 (base_final), left_down_1, right_down_1, mouse_short_1, mouse_click_short_1, base_clean_1 | |
| 머리 | head/excited = head_excited_1 (e246@0:12), head/overheat = head_overheat_1 (e251@0:45) | |
| idle `shisa_study` | scene_study_a_1 -> scene_study_b_1, 2단계 교대 + 숨쉬기 | |
| milestone `shisa_pass` | scene_pass_a_1 -> scene_pass_b_1, 2장 교대 + hop | |
| gesture `shisa_ramen` | scene_ramen_a_1 -> scene_ramen_b_1, 2단계 교대 + 숨쉬기 | 흰 두건 속이 base 로 비쳐 보여서 slice.py changed() 가 둘러싸인 속을 채우게 고쳤다 |
| ambient `shisa_hum` | fx/shisa_notes = behavior_notes_1, anchor [860, 218] | |
| 소품 | ramen = prop_ramen_1 | |

- 머리 마스크는 `art/scripts/silhouette_head.py --name shisa` 로 실루엣까지 넓혔다. `build.sh --remask` 를 하면 다시 돌려야 한다.
- 움직임 폭은 motion_qa T3(12px) 에 맞춰 hop 2.4pt 이하, 흥분 단계 진입 hop 은 기본 팝과 겹쳐서 1.6pt. `pop_scale` 은 기본값을 쓴다 (올리면 T3 FAIL).
- 2026-11-18 `--check` PASS, motion_qa PASS (출력 폴더를 따로 잡아 시사/카니만 돌림).

## 확인

- `typing-pet --check characters/shisa` 통과
- `typing-pet --character-dir characters/shisa --render-all <폴더>` 의 `bhv-*.png`
- `scripts/motion_qa.sh`
