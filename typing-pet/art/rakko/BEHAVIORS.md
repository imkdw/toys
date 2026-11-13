# 랏코 행동 시트

토벌 랭킹 1위 상위 랭커 해달. 근거는 실제 애니(한국어 더빙) 프레임이고 `e<회차>@<분:초>` 는 회차 안의 시각이다.
대사는 더빙 음성을 받아 적은 것이다 (whisper, 회차 안 시각).

## 성격 한 줄

말수 적고 침착한 최강 헌터. 반응은 "흠." 한마디와 작은 끄덕임 정도인데, 디저트(늘 먹는 딸기 파르페) 앞에서만 행복하게 풀어진다.

## 생김새 근거 (`ref/real/`)

| 크롭 | 근거 | 본 것 |
|---|---|---|
| `e51_0044_standing.png` | e51@0:44 | 치이카와 옆에 선 전신. 치이카와보다 크다. 등에 칼, 초록 주머니 |
| `e52_0017_face_calm.png` | e52@0:17 | 평소 얼굴. 머리 윗선은 끊긴 짧은 털 선, 옆머리 위쪽의 짙은 갈색 귀, 왼쪽 이마 별 흉터 |
| `e52_0005_face_smile.png` | e52@0:05 | 웃는 얼굴 클로즈업 |
| `e54_0043_table.png` | e54@0:43 | 찻집 탁자에 앉은 모습. 앞발이 짙은 갈색, 흰 목도리가 망토처럼 어깨를 덮음 |
| `e179_0041_counter.png` | e179@0:41 | 가게 카운터 앞에 앉은 모습 |

## 대표 동작

근거 크롭은 `art/rakko/ref/behavior/`, 장면 캡처는 `art/rakko/ref/scene/`. behavior 이름은 다른 캐릭터와 겹치지 않게 `rakko_` 로 시작한다.

| 동작 (behavior) | 참고 회차/프레임 id | 연결할 cue | 표현 (layer_mode, motion, fx, head) | 필요한 그림 |
|---|---|---|---|---|
| 늘 먹던 딸기 파르페 (`rakko_parfait`) | e54@0:43 "늘 먹던 걸로!" 주문, e54@0:50 숟가락, e54@0:53 볼 감싸고 음미 | `idle` | full, 2단계 크로스페이드(숟가락 -> 볼 감싸기) + 숨쉬기 + "늘 먹던 걸로!" 말풍선 pop_in | full/parfait_a, full/parfait_b, fx/order |
| 랭커의 일격 (`rakko_slash`) | e51@0:17 칼을 휘둘러 노란 빛줄기로 베기, e53@0:12 토벌 성적 랭킹 1위 게시판 | `milestone` | full, 2장(칼 들기 -> 베기) | full/slash_a, full/slash_b |
| 핫코코아 한 모금 (`rakko_cocoa`) | e115@0:38 코코아 컵 들기, e115@0:40 마시고 따뜻한 숨 + 코 홍조 | `gesture` (과열이 식을 때) | full, 2장 교대 + 김 fx float | full/cocoa_a, full/cocoa_b, fx/steam |
| 번뜩 (`rakko_bolt`) | e117@0:09 전단지 읽다가 번개 번쩍 "아! 이럴 수가!" | `wake` | overlay, 번개 fx flash | fx/bolt |
| 좋았어! (`rakko_joa`) | e117@0:15 "좋았어!", 웃는 얼굴 e52@0:05 | `level:excited` | overlay, head/excited + "좋았어!" 말풍선 | head/excited, fx/joa |
| 진지해짐 (`rakko_focus`) | e53@0:16 굳은 눈썹, e54@0:22 "흠..." | `level:overheat` | overlay, head/overheat(찌푸린 진지한 얼굴) + 머리 nod 천천히 | head/overheat |
| 빨리 가야겠군! (`rakko_hurry`) | e117@0:08~0:12 "빨리 가야겠군!" 하고 바로 출발 | `burst` | overlay, 말풍선 pop_in + 몸 sway 빠르게 | fx/hurry |
| 디저트 생각 (`rakko_dream`) | e115@0:44 마시멜로 피자 생각 풍선, e117@0:17 디저트 전단지 | `ambient` | overlay, 파르페 생각 풍선 float + 머리 nod | fx/dream |
| 흠. 감사. (`rakko_thanks`) | e53@0:16~0:19 칭찬받고 "흠. 감사." | `prop` (책상 소품 누름) | overlay, 말풍선 pop_in + 머리 nod 한 번 | fx/thanks |

- `idle`, `milestone`, `gesture` 각 1개, 그 밖의 cue 6개 (wake, level:excited, level:overheat, burst, ambient, prop).
- 9개 모두 motion, 움직이는 fx, 숨쉬기, 크로스페이드 중 하나 이상을 쓴다.

## 공통 머리 단계를 이 캐릭터는 어떻게 보여주나

애니에 나온 표정만 그린다. picks.json `_expressions` 에 근거와 함께 적은 것만 엔진이 그리고, 나머지 단계는 head/normal 로 간다.

| excited | dizzy | overheat | sleepy |
|---|---|---|---|
| 입 벌린 웃음 (e52@0:05) | 없음 (애니에 어지러운 랏코가 없음, head/normal) | 찌푸린 진지한 얼굴 (e53@0:16) | 없음 (조는 랏코 프레임을 못 찾음, head/normal) |

선글라스(egg), 소용돌이 눈, 김 나는 얼굴 같은 지어낸 표정은 만들지 않는다.

## 성격 타이밍

| 값 | 기본 | 랏코 | 이유 |
|---|---|---|---|
| `_anim.pop_scale` (단계 오를 때 팝) | 1.04 | 1.02 | 침착해서 반응이 작다 |
| `_timing.level_up` (단계 오르는 점수) | [12, 22, 32] | [15, 26, 38] | 최강 헌터라 웬만큼 쳐서는 흥분하지 않는다 |
| `_anim.glow_ms` / `_timing.paw_down_ms` (앞발 누름) | 220 / 120 | 200 / 100 | 군더더기 없이 빠르고 정확하게 친다 |
| `_timing.wake_after_s` | 15 | 20 | 잠깐 쉬는 정도로는 반응하지 않는다 |
| `_timing.burst_keys` | 9 | 11 | 웬만한 속도는 평소 속도 |
| `_timing.ambient_every_s` (최소 60) | [60, 180] | [90, 240] | 말수가 적다 |

## 책상 소품

**랏코의 초록 주머니 (끈 주머니)**

- 근거: e51@0:44 (왼손에 들고 다님), e52@0:15 (식당 자리 옆에 둠), e54@0:42~0:55 (찻집 자리 옆에 내려놓고 파르페를 먹음), e213@0:33 (서서 먹는 국수 가게에서 옆에 둠). 크롭 `ref/behavior/pouch_e54_0042.png`, `ref/behavior/pouch_closeup_e54_0053.png`.
- 그리는 법: 컵 정도 크기로 책상 위에 혼자 서 있는 둥근 천 주머니. 아래는 둥글고 납작하게 퍼져 앉아 있고, 위쪽 목을 끈으로 졸라 매서 천이 위로 서너 갈래 뾰족하게 삐죽 솟는다.
  색은 탁한 올리브 그린 (#A3A86C 근처, 그림자 없이 평평), 얇은 짙은 갈색(#3A2A25) 외곽선, 몸통에 짧은 짙은 초록 해칭 선 몇 개(천 질감).
  매듭 자리에 흰(크림색) 리본 나비매듭 (e54) 이 있다. 글자, 무늬, 얼굴 없음.
- 반응: 이 소품을 누르면 `rakko_thanks` ("흠. 감사." 말풍선 + 끄덕임).

## 지금 상태와 남은 일 (2026-11-13)

2026-11-13 에 `stopgap/regen.sh` 로 앞발/base_clean/머리/장면 첫 프레임까지 생성하고 picks.json 에 넣었다.
`scene_cocoa_a` 부터 codex 사용량 한도에 다시 걸렸다 ("try again at Nov 14th, 2026 1:23 AM").
설치본은 아직 `characters/_wip_rakko` (메뉴에 안 보임).

| 항목 | 지금 | 남은 일 |
|---|---|---|
| 앞발 누름/center/mouse/click | 생성본 (left_down_1, right_down_1, right_center_1, mouse_1, mouse_click_1) | 없음 |
| base_clean | 생성본 base_clean_1 | 없음 |
| 머리 | head/excited = head_excited_1 (e52@0:05), head/overheat = head_overheat_1 (e53@0:16, 덜 찌푸린 쪽). `rakko_joa`/`rakko_focus` 에 연결 | 없음 |
| idle `rakko_order` | overlay "늘 먹던 걸로!" PIL 말풍선 + 끄덕임 | 고른 첫 프레임 `scene_parfait_a_1` 에서 `41_scene_parfait_b` 생성, full `rakko_parfait` 로 바꾸기 |
| milestone `rakko_slash` | overlay PIL 베기 빛줄기 | 고른 첫 프레임 `scene_slash_a_1` 에서 `43_scene_slash_b` 생성, full 로 바꾸기 |
| gesture `rakko_puff` | overlay PIL 입김 + squash | `44_scene_cocoa_a` 생성 (실패한 곳), 그 다음 `45_scene_cocoa_b`, `67_behavior_steam`, full `rakko_cocoa` 로 바꾸기 |
| ambient `rakko_nod` | 머리 끄덕임 | `66_behavior_dream` 생성, `rakko_dream` 으로 바꾸기 |
| 말풍선/번개 | PIL (`*_pil`) | `60_`~`63_`, `65_` 생성 후 picks `_fx` 의 src 교체 |
| `rakko_focus` 의 "흠..." 말풍선 | 뺐다 (생성 프롬프트가 없고 표대로 head/overheat + nod) | 없음 |

머리 마스크는 `art/scripts/silhouette_head.py --name rakko` 로 몸 전체 실루엣까지 넓혔다. `build.sh --remask` 를 하면 다시 돌려야 한다.

## 확인

- `typing-pet --check characters/rakko` 통과
- `typing-pet --character-dir characters/rakko --render-all <폴더>` 의 `bhv-*.png`
- `typing-pet --character-dir characters/rakko --render-timeline <폴더> --scenario <wake|burst|levels|keys>`
