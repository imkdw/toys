# 카니 행동 시트

게 집게 머리띠를 한 헌책방 주인 (古本屋, 카니). 로컬 더빙 원본(254화까지)에는 회색 모브로만 나와서 (e057 헌책 좌판 뒷모습),
근거는 아래 두 영상에서 가져왔다. 받은 파일은 저장소에 두지 않았다.

- `pv2@<분:초>`: 공식 2탄 PV (めざましテレビチャンネル "アニメ『ちいかわ』第2弾PV", YouTube TET5lDDhetM)
- `c2@<분:초>`: 본편 클립 (YouTube M34v8T0UNSw, 모몽가와 헌책방 장면, 일본어판 341화 무렵)

말풍선 대사는 쓰지 않는다 (한국어 더빙이 없음).

## 성격 한 줄

조용하고 다정한 책벌레. 평소엔 차분히 책을 읽고, 기쁘면 눈물이 맺힐 만큼 좋아하며, 마음먹으면 눈썹을 세우고 주먹을 쥔다.

## 생김새 근거 (`ref/real/`)

| 크롭 | 근거 | 본 것 |
|---|---|---|
| `pv2_0054_front.png` | pv2@0:54 | 정면 상반신. 아주 연한 분홍 몸, 머리 위쪽을 덮는 분홍 띠, 양쪽 위 분홍 집게 |
| `pv2_0050_face.png` | pv2@0:50 | 얼굴 클로즈업. 눈 위 짧은 두 줄 눈썹, 작은 눈, ω 입, 빗금 볼터치 |
| `pv2_0058_card.png` | pv2@0:58 | 캐릭터 소개 컷 전신. 초록 책을 들고 있음 |
| `c2_0049_seated.png` | c2@0:49 | 좌판 방석에 앉은 정면 전신 |

## 대표 동작

근거 크롭은 `ref/behavior/`, 장면 캡처는 `ref/scene/`. behavior 이름은 `kani_` 로 시작한다.

| 동작 (behavior) | 참고 프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 책 읽기 (`kani_read`) | pv2@0:58 초록 책, c2@1:42 초록 책을 펴고 읽음 | `idle` | full, 2단계 교대(읽기 -> 책장 넘기기) + 숨쉬기 | full/kani_read_a, _b |
| 기쁨의 눈물 (`kani_joy`) | c2@0:01 눈 감고 웃으며 눈물 한 방울, 뒤에 노란 빛 | `milestone` | overlay, head/joy + 몸 hop + 반짝이 pulse | head/joy, fx/kani_sparkle |
| 후유 (`kani_phew`) | c2@1:49 머리 옆 땀 자국 | `gesture` (과열이 식을 때) | overlay, 땀 fx fall + 몸 squash | fx/kani_sweat |
| 와아 (`kani_wow`) | pv2@0:53 눈 크게 뜨고 입 벌려 웃음 | `level:excited` | overlay, head/excited + 몸 hop | head/excited |
| 결심 (`kani_resolve`) | pv2@0:55 눈썹을 세우고 주먹 쥠 | `level:overheat` | overlay, 몸 tremble + 머리 nod (표정은 head/normal, 아래 "지금 상태") | 없음 |
| 고개 들기 (`kani_look`) | pv2@0:50~0:52 고개를 돌려 올려다봄 | `wake` | overlay, 머리 nod 1회 | 없음 |
| 책과 꽃 (`kani_bookmark`) | pv2@0:58 소개 컷의 책/꽃 아이콘 | `ambient` | overlay, 아이콘 fx float + 몸 sway | fx/kani_bookmark |
| 헌책 고맙습니다 (`kani_thanks`) | pv2@0:52 좌판의 책 상자, 표정 c2@0:01 | `prop` | overlay, head/joy + 머리 nod | head/joy |

- `idle`, `milestone`, `gesture` 각 1개, 그 밖의 cue 5개.
- 8개 모두 motion, 움직이는 fx, 숨쉬기, 크로스페이드 중 하나 이상을 쓴다.

## 공통 머리 단계를 이 캐릭터는 어떻게 보여주나

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 눈 크게 뜨고 입 벌려 웃음 (pv2@0:53) | 없음 (head/normal) | 없음 (근거 pv2@0:55 는 있으나 그림이 달라서 뺌, head/normal) | 없음 (head/normal) |

그 밖의 허용 표정: head/joy (c2@0:01). 지어낸 표정은 만들지 않는다.

## 성격 타이밍

| 값 | 기본 | 카니 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` | 1.04 | 기본 | |
| `_timing.level_up` | [12, 22, 32] | [14, 24, 34] | 차분해서 쉽게 달아오르지 않는다 |
| `_anim.glow_ms` / `_timing.paw_down_ms` | 220 / 120 | 240 / 130 | 살살 누른다 |
| `_timing.wake_after_s` | 15 | 18 | 책에 빠져 있다가 천천히 고개를 든다 |
| `_timing.burst_keys` | 9 | 9 | 기본 |
| `_timing.ambient_every_s` | [60, 180] | [80, 200] | 조용하다 |

## 책상 소품

**헌책 상자 (`bookbox`)**

- 근거: pv2@0:52 (좌판에 놓인 책 상자), c2 의 좌판 장면. 크롭 `ref/behavior/bookbox_pv2_0052.png`.
- 그리는 법: `art/_props/prompts/105_prop_bookbox.txt`. 낮은 나무 상자에 색색의 책이 꽂혀 있음, 글자 없음.
- 반응: 누르면 `kani_thanks`.

## 지금 상태 (2026-11-18 끝남)

모든 그림이 생성본이다. 설치본은 `characters/kani`.

| 항목 | 쓰는 그림 | 메모 |
|---|---|---|
| base, 앞발/마우스, base_clean | base_2 (base_final), left_down_1, right_down_1, mouse_short_1, mouse_click_short_1, base_clean_1 | |
| 머리 | head/excited = head_excited_1 (pv2@0:53), head/joy = head_joy_1 (c2@0:01) | |
| idle `kani_read` | scene_read_a_1 -> scene_read_b_1, 2단계 교대 + 숨쉬기 | |
| milestone `kani_joy` | head/joy + hop + fx/kani_sparkle (behavior_sparkle_1, anchor [195, 228]) | |
| gesture `kani_phew` | fx/kani_sweat (behavior_sweat_1, anchor [816, 216]) + squash | |
| ambient `kani_bookmark` | fx/kani_bookmark (behavior_bookmark_1, anchor [156, 258]) + sway | |
| 소품 | bookbox = prop_bookbox_1 | |

- **결심한 얼굴(head/overheat)은 뺐다.** 후보 4장(프롬프트를 고쳐 한 번 더 생성) 모두 눈이 반쯤 감겨 능글맞게 나와 pv2@0:55 와 달랐다.
  `kani_resolve` 는 head/normal 로 몸 tremble + 머리 nod 만 한다. 다시 그리려면 `prompts/23_overheat.txt`, 후보는 `raw/head_overheat_1/2.png`.
- 머리 마스크는 `art/scripts/silhouette_head.py --name kani` 로 실루엣까지 넓혔다. `build.sh --remask` 를 하면 다시 돌려야 한다.
- 움직임 폭은 motion_qa T3(12px) 에 맞춰 hop 2.0pt 이하, 흥분 단계 진입 hop 은 1.6pt. `pop_scale` 은 기본값.
- 2026-11-18 `--check` PASS, motion_qa PASS.

## 확인

- `typing-pet --check characters/kani` 통과
- `typing-pet --character-dir characters/kani --render-all <폴더>` 의 `bhv-*.png`
- `scripts/motion_qa.sh`
