# characters

캐릭터 하나당 폴더 하나. `_` 로 시작하는 폴더(`_test`)만 git 에 올리고, 나머지는 올리지 않는다.

```
characters/<name>/
├── manifest.toml   메뉴 이름(title), 레이어 목록, 마스크, 발끝 좌표, 머리/몸 기준점, 마우스 연출 축([swing])
│                   애니 장면([[scene]]), 캐릭터 고유 행동([[behavior]]), 허용 표정([expressions]), 책상 소품([desk])
│                   ([timing] [anim] [keys] 로 기본값 덮어쓰기)
├── layers/
│   ├── base.png             책상(키보드/마우스만)과 캐릭터. 책상 소품은 _props 에서 따로 얹는다
│   ├── base_clean.png       (선택) 캐릭터를 지운 책상. 몸 전체 움직임으로 드러나는 자리를 채운다
│   ├── head/*.png           머리 변형 (마스크 안을 교체)
│   ├── paw_l/*.png          왼앞발 자세
│   ├── paw_r/*.png          오른앞발 자세 (mouse_bg: 마우스 자세에서 팔을 치운 바탕)
│   ├── fx/*.png             선글라스 단독, 마우스 잡은 팔(mouse_arm), 장면/행동 소품(bubble, zzz, 말풍선), 플래시(white) (일반 합성)
│   ├── full/*.png           장면 프레임(full/<장면>_a, _b ..)과 제스처 같은 전체 프레임
│   └── mask/*.png           head, paw_l, paw_r 영역 마스크, body(머리+몸통, 앞발/키보드 제외)
└── baseline/       조합 시트 기준 이미지 (scripts/verify.sh)

characters/_props/  모든 캐릭터가 같이 쓰는 책상 소품 (art/scripts/make_props.py 가 만든다)
├── props.toml      [[prop]] 소품(자리, 누를 때 움직임), [[job]] 직군별 소품 묶음
└── layers/<id>.png 캔버스 크기, 소품이 제자리에 그려진 그림
```

모든 레이어는 같은 크기의 정사각형 캔버스다. `art/scripts/build.sh` 가 생성 원본에서 만든다.
앞발/제스처 레이어는 base 와 실제로 달라진 곳만 변형 그림이고 나머지(책상, 키보드)는 base 와 같다.
`paw_r/mouse_bg` + `fx/mouse_arm` + `[swing]` 이 있으면 마우스를 움직이는 동안 팔만 root 에서 tip 쪽으로 휘며 흔든다.
없으면 `paw_r/mouse` 한 장으로 가만히 있다.
메뉴바의 **캐릭터** 메뉴에 `_` 로 시작하지 않는 폴더가 manifest 의 `title`(없으면 폴더 이름) 순으로 나오고, 고르면 바로 바뀌며 설정에 저장된다.
설정에서 캐릭터를 지정하지 않았거나 지정한 폴더가 없으면 그 목록의 첫 번째(지금은 우사기)를 쓴다.

## 애니에 나온 표정만 ([expressions])

머리 레이어는 manifest `[expressions]` 에 애니 근거(회차/프레임 id)와 함께 적힌 것만 그린다. `head/normal` 은 항상 허용.
여기 없는 머리는 파일이 있어도 머리 단계(excited, dizzy, overheat, sleepy), 선글라스 이스터에그, behavior 의 머리 바꾸기 어디서도
그리지 않고 `head/normal` 로 대신한다 (`src/anim.rs` `Layers::resolve`). 선글라스 소품(`fx/sunglasses`)도 `head/sunglasses` 가 허용될 때만 나온다.

```toml
[expressions]
"head/excited" = "e066-007, e015-014 '우라!' 입 크게 벌린 얼굴"
"head/haa" = "e036-024 '하아?'"
```

picks.json 에 `"_expressions": {...}` 로 적으면 slice.py 가 넣는다. 근거 없이 그린 표정(땀 흘리는 멍한 얼굴, 김 나는 화난 얼굴,
픽셀 선글라스 등)은 넣지 않는다. 단계 반응은 표정 대신 motion/fx behavior 로 한다.
`--check` 는 behavior 가 허용되지 않은 머리를 쓰거나 근거가 비어 있으면 실패한다.

| 캐릭터 | 허용 표정 |
|---|---|
| 우사기 | excited (e066-007, e015-014), haa (e036-024) |
| 치이카와 | overheat = 울먹임 (e185-028) |
| 하치와레 | excited (e169-007), overheat = 눈 감고 웃음 (e112-015, e010-005), tongue (e169-007) |

## 책상 소품과 직군 ([desk], characters/_props)

책상 위 물건은 캐릭터 그림에 넣지 않고 공용 소품 레이어로 얹는다. 자리는 세 곳이다 (캔버스 px, 바닥 가운데).

| 자리 | foot | 누가 채우나 |
|---|---|---|
| `left_back` (키보드 왼쪽 뒤) | [162, 784] | 직군 |
| `left_front` (키보드 왼쪽 앞) | [212, 896] | 직군 |
| `right_back` (마우스 오른쪽) | [858, 786] | 캐릭터 소품 (`[desk] item`) |

메뉴바 **직군 (책상 장비)** 에서 고르면 바로 바뀌고 설정(`job`)에 저장된다. 처음엔 `기본`.

| 직군 | left_back | left_front |
|---|---|---|
| 기본 | 연필꽂이 | 과자와 별 쿠션 (예전 그림에서 잘라 옴) |
| 개발자 | 커피 머그 | 고무 오리 |
| 디자이너 | 붓 꽂은 병 | 팔레트 |
| 작가 | 책 더미 | 노트와 만년필 |
| 학생 | 스탠드 | 연필과 지우개 |
| 기획자 | 탁상 달력 | 포스트잇 |

캐릭터 소품: 우사기 푸딩, 치이카와 브로콜리, 하치와레 카메라 (새 캐릭터는 행동 시트의 `## 책상 소품`).

**상호작용**: 소품을 누르면 (끌기 대신) 소품이 `[[prop.tap]]` 움직임(hop, tremble, squash 등, 없으면 작게 통통)을 하고
캐릭터에게 `prop` cue 를 알린다. 캐릭터는 `on = "prop"` behavior 로 반응한다 (`--check` 필수 규칙).

```toml
[desk]
item = "pudding"            # props.toml 의 id

[desk.hide]                 # slice.py 가 계산: 이 장면 프레임이 보이는 동안 이 자리 소품을 숨김 (캐릭터 팔이 덮는 곳)
"full/staff_b" = ["right_back"]
```

새 소품/직군 추가:

1. `art/_props/prompts/<번호>_prop_<이름>.txt` 에 물건 설명과 자리 문장을 쓰고
   `art/scripts/gen.sh -c _props -i art/_desk/desk_empty.png -n 2 <파일>` (빈 책상에 그 물건만 그려 넣은 그림).
2. `art/_props/props.json` 의 `props` 에 `{"title", "slot", "src": "<고른 후보>", "tap": [...]}`, 직군은 `jobs` 에.
3. `cd art/scripts && ../.venv/bin/python make_props.py` (빈 책상과 달라진 덩어리를 잘라 자리 크기에 맞춰 놓는다).

예전처럼 소품까지 그려 넣은 캐릭터는 picks.json 에 `"desk_empty": "<그 캐릭터의 소품만 지운 책상 후보>"` 를 적으면
slice.py 가 모든 레이어에서 소품 자리(`art/_desk/item_mask.png`)를 빈 책상으로 바꾼다.
`art/_desk/` (빈 책상, 소품 자리 마스크, 소품을 지운 우사기)는 `art/scripts/make_desk.py` 가 만든다. 새 캐릭터는 `art/_desk/usagi_noitems.png`
(키보드/마우스만 있는 책상)를 base 입력으로 쓰면 처음부터 소품이 없다.

`--render-all`, `--render-timeline`, `--render-state` 에 `--job <id>` 를 주면 그 직군 소품을 얹어 그린다.

## 애니 장면 ([[scene]])

캐릭터마다 실제 애니 장면을 따라 한 전체 프레임 연출이 있다. 프레임 여러 장을 `frame_ms` 마다 돌려 그린다.

```toml
[[scene]]
name = "eat"
trigger = "idle"          # gesture: 과열이 식을 때, idle: 한동안 입력 없음, milestone: 키 N번째
frames = ["full/eat_a", "full/eat_b"]
frame_ms = 450
duration_ms = 6000        # 없으면 [timing] 의 gesture_ms / idle_scene_ms / milestone_ms
```

- idle 은 입력이 `idle_scene_s`(20초) 없으면 한 번 틀고, 입력이 오면 바로 끝난다.
- milestone 은 키를 `milestone_keys`(300) 번 칠 때마다 튼다.
- idle/milestone 장면이 없는 캐릭터는 그 연출을 하지 않고, gesture 장면이 없으면 `full/gesture` 한 장을 쓴다.
- 같은 조건의 장면이 여러 개면 돌아가며 고른다.

움직임이 더 필요한 장면은 `frames` 대신 단계(`step`), 숨쉬기(`breathe`), 소품(`fx`)을 쓴다. 예 (단계 여러 개를 쓰는 수면 장면):

```toml
[[scene]]
name = "sleep"
trigger = "idle"
duration_ms = 0           # idle 장면만 0 가능: 입력이 올 때까지 계속

[scene.breathe]           # 머리 마스크 안을 천천히 오르내림
period_ms = 3600
pt = 1.8

[[scene.step]]            # 단계를 순서대로 반복. 단계 끝 blend_ms 동안 다음 프레임으로 크로스페이드
frame = "full/sleep_a"    # 비렘: 감은 눈
ms = 7000
blend_ms = 700

[[scene.step]]
frame = "full/sleep_rem_a"  # 렘: 반쯤 뜬 눈, 중얼거림 (rem_b 와 번갈아)
ms = 1400
blend_ms = 250

[[scene.fx]]
layer = "fx/bubble"       # 콧방울
kind = "pulse"            # anchor 기준으로 scale[0]~scale[1] 사이를 커졌다 작아짐
anchor = [524, 498]       # 캔버스 px
period_ms = 3600
scale = [0.3, 1.0]
on = ["full/sleep_a"]     # 이 프레임이 보일 때만 (블렌드 중엔 같이 흐려짐)

[[scene.fx]]
layer = "fx/zzz"
kind = "float"            # anchor 에서 rise_pt 만큼 떠오르며 나타났다 사라짐
anchor = [714, 240]
period_ms = 3000
rise_pt = 14
on = ["full/sleep_a"]
```

소품 레이어는 picks.json 의 `"_fx": {"<이름>": {"src": "<소품이 있는 후보>", "ref": "<소품만 없는 후보>"}}` 로 정하면
build.sh 의 `art/scripts/extract_fx.py` 가 두 그림의 차이만 잘라 `fx/<이름>` 으로 만든다 (경계 상자를 찍어 주니 anchor 를 잡을 때 참고).
장면 움직임은 `typing-pet --render-timeline <폴더> --scenario idle --secs 42 --start 19` (milestone 은 `--scenario milestone --secs 15 --start 11`)로 프레임을 뽑아 확인한다.

| 캐릭터 | idle | milestone | gesture (과열이 식을 때) |
|---|---|---|---|
| 우사기 | 책상에서 졸기: 감은 눈 + 콧방울/Zzz + 숨쉬기 (만화 2023-02-06, 2026-09-09) | "야하!" 외치는 얼굴 + 말풍선 (e204) | 해골 지팡이 휘두르며 흔들기 (e036) |
| 치이카와 | 브로콜리 먹고 감동해서 울먹임, 숨쉬기 (e004) | 제초 검정 5급 합격증 들어 보이며 통통 (e060, e224) | 눈물 흘리며 부들부들 (e224, e185) |
| 하치와레 | 코스 요리 책 읽기, 숨쉬기 (e201) | 카메라로 사진 찍기 + 플래시 (e037, e216) | 기타 치며 노래 + 음표, 흔들기 |

장면 밖의 고유 행동 (`[[behavior]]`, 자세한 근거는 `art/<이름>/BEHAVIORS.md`):

| 캐릭터 | 성격 | 행동 (cue) |
|---|---|---|
| 우사기 | 시끄럽고 몸이 먼저 나감 | "우라!" 말풍선 (level:excited), 프루루루 고속 진동 (level:dizzy 동안), 과열에 부르르 (level:overheat), "하아?" (wake), 귀 쫑긋 (ambient) |
| 치이카와 | 겁 많고 잘 욺 | 울먹이며 깜짝 + 땀방울 (wake), 부들부들 (level:dizzy 동안), 울먹이는 과열 얼굴 + 눈물 (level:overheat), 브로콜리 누르면 감동 (prop) |
| 하치와레 | 밝고 낙천적 | "어떻게든 되겠지!" 말풍선 + 땀 흘리며 웃는 과열 얼굴 (level:overheat), 음표 흥얼거리기 (level:excited 동안), 혀 내밀고 웃기 (ambient) |

말풍선 대사는 한국어 더빙 기준이다.
괄호는 참고한 애니 회차 (`art/<이름>/ref/scene/` 에 크롭). 만드는 법:

1. `art/<이름>/prompts/4x_scene_<장면>_a.txt` (첫 프레임, base 에서 편집), `_b.txt` (둘째 프레임, 고른 첫 프레임에서 편집).
2. `art/scripts/gen.sh -c <이름> -i art/<이름>/raw/base_final.png -r <생김새 참고> -p art/<이름>/ref/scene/<장면>.png -n 2 4x_scene_<장면>_a.txt`
   (`-p` 는 따라 할 장면 캡처. 둘째 프레임은 `-i` 에 고른 첫 프레임을 준다.)
3. `picks.json` 에 `"full/<장면>_a": "<후보>"` 와 `"_scenes": {"<장면>": {"trigger": "idle", "frame_ms": 450}}` 를 적고 build.sh.

## 캐릭터 고유 행동 ([[behavior]])

엔진은 "무슨 일이 일어났는지(cue)" 만 알리고, 어떻게 반응할지는 캐릭터가 `[[behavior]]` 로 정한다. `src/` 에는 캐릭터 이름이 없다.
`[[scene]]` 은 `layer_mode = "full"` 인 behavior 로 읽히므로 예전 manifest 도 그대로 동작한다.

| cue | 언제 | 조절 ([timing]) |
|---|---|---|
| `idle` `milestone` `gesture` | 위 장면과 같음 | `idle_scene_s`, `milestone_keys`, `gesture_cooldown_s` |
| `level:<normal\|excited\|dizzy\|overheat\|sleepy>` | 머리 단계가 바뀔 때 (`when = "hold"` 면 그 단계인 동안 반복) | `level_up`, `level_down`, `sleepy_s` |
| `egg` | 선글라스 이스터에그 (hold 가능) | `easter_egg_repeat` |
| `wake` | 오래 쉬다가 첫 입력 | `wake_after_s` (15) |
| `burst` | 짧은 시간에 몰아친 키 | `burst_window_ms` (1000), `burst_keys` (9) |
| `key:<enter\|backspace\|space\|tab\|escape\|delete>` | 그 키를 누를 때 (키 코드만 비교하고 저장하지 않음) | |
| `ambient` | 평소 상태에서 무작위 간격 (장면 중엔 건너뜀) | `ambient_every_s` ([60, 180]) |
| `prop` | 책상 소품을 누를 때 | |

```toml
[[behavior]]
name = "ura"
on = "level:excited"        # cue
when = "enter"              # enter: 한 번 | hold: 그 상태인 동안 duration_ms 재생, cooldown_s 쉬기를 반복 (level:*, egg 만)
chance = 1.0                # 0~1
cooldown_s = 20             # 한 번 한 뒤 다시 하기까지
priority = 20               # 같은 층(full/overlay)에서 겹치면 큰 쪽, 같으면 나중 것. 기본 10
layer_mode = "overlay"      # overlay: 평소 합성 위에 얹음 | full: 전체 프레임 (없으면 frames/step 이 있을 때 full)
duration_ms = 900           # 없으면 1200 (idle/milestone/gesture 의 full 은 [timing] 값)

[[behavior.motion]]
target = "body"             # body: 몸 마스크 안(책상/앞발은 제자리) | head: 머리 마스크 안 | fx:<소품> (예: fx:ura)
kind = "hop"                # hop, tremble, sway, nod, squash
pt = 6                      # 폭 (pt). 없으면 종류별 기본값
period_ms = 450             # 한 번 하는 시간
count = 1                   # 0 이면 behavior 가 끝날 때까지 반복 (시작/끝 100ms 는 서서히)
delay_ms = 0
squash = 0.05               # hop 착지/squash 의 납작한 정도

[[behavior.fx]]             # 장면 소품과 같은 표 + 종류 추가
layer = "fx/ura"
kind = "pop_in"             # float, pulse, pop_in(말풍선), fall(눈물/땀), flash(번쩍)
anchor = [760, 300]
period_ms = 800             # pop_in/flash: 보이는 시간 (한 번), 나머지: 한 주기 (반복)
delay_ms = 0
scale = [0.3, 1.0]          # pulse: 작을 때/클 때, pop_in: 시작/끝 크기
rise_pt = 14                # float: 떠오르는 높이, fall: 떨어지는 거리

[behavior.head]             # overlay 동안 머리 바꾸기 (시작/끝은 원래 머리와 크로스페이드)
layer = "head/haa"          # 또는 frames = ["head/ear_a", "head/ear_b"], frame_ms = 150
```

- 우선순위: full > overlay > 평소 합성. full 이 보이는 동안 overlay 는 그리지 않는다.
- 같은 cue 에 behavior 가 여럿이면 chance/cooldown 을 통과한 것 중에서 돌아가며 고른다.
- 몸 전체 움직임은 `mask/body` 안만 옮기고, `base_clean` 이 있으면 드러나는 자리를 그걸로 채운다. 없으면 이동 폭을 2pt 로 제한한다.
- 메뉴바 **반응 줄이기** 를 켜면 장면(idle/milestone/gesture) 밖의 behavior 를 하지 않는다.

picks.json 에는 `"_behaviors": {"<이름>": {"on": "...", "motion": [...], "fx": [...], "head": {...}}}` 로 적는다
(full 인데 frames 가 없으면 `full/<이름>_a, _b ..`). 성격 타이밍은 `"_timing"`, `"_anim"` 으로 `[timing]`, `[anim]` 에 들어간다.
`"base_clean": "<후보>"` 를 적으면 캐릭터를 지운 책상 레이어가 생기고, behavior 가 `fx/white` 를 쓰면 base 실루엣 모양 흰색이 생긴다.

확인:

- `typing-pet --check characters/<이름>`: 아래 규칙. `scripts/verify.sh` 가 `_` 로 시작하지 않는 모든 캐릭터에 돌린다.
- `typing-pet --character-dir characters/<이름> --render-all <폴더>`: 조합 시트 + behavior 마다 `bhv-<이름>.png`.
- `typing-pet --render-timeline <폴더> --scenario <wake|burst|levels|keys|ambient>`: `timeline.json` 의 `started`/`playing` 에 behavior 이름이 남는다
  (시드 고정이라 같은 시나리오를 캐릭터끼리 비교할 수 있다).

새 캐릭터 규칙 (`--check`, `src/config.rs` `validate_character`):

| 규칙 | 이유 |
|---|---|
| `idle`, `milestone`, `gesture` 에 behavior(또는 scene) 가 각각 1개 이상 | 조용히 꺼지는 것 방지 |
| 그 밖의 cue 에 behavior 3개 이상 | 평소에도 그 캐릭터다운 순간 |
| 움직임(motion, fx, 숨쉬기, 크로스페이드)이 있는 behavior 가 절반 이상 | 2장 교대만 있는 장면 방지 |
| 다른 캐릭터와 behavior 이름, 레이어 파일이 겹치지 않음 | 복사해 놓고 이름만 바꾼 캐릭터 방지 |
| `ambient_every_s` 최소 60초 | 시끄러움 방지 |
| 쓰는 레이어가 모두 있고 cue 이름 오타 없음 | manifest 검사 |
| behavior 가 쓰는 머리는 `[expressions]` 에 애니 근거가 있음 | 애니에 없는 표정 방지 |
| `[desk] item` 소품이 있고 `prop` cue behavior 가 1개 이상 | 책상 소품 상호작용 |

`_test` 는 이 규칙에서 빠지지만 모든 cue/motion/fx 종류를 쓰는 가짜 behavior(`t_*`)가 있어 엔진 테스트(`tests/behavior.rs`)와 기준 이미지에 쓴다.

## 새 캐릭터 추가 순서

생성 원본, 프롬프트, 마스크는 `art/<이름>/` 에 캐릭터별로 둔다.

1. **행동 시트**: `art/_template/BEHAVIORS.md` 를 `art/<이름>/BEHAVIORS.md` 로 복사해 성격, 대표 동작 6개 이상(참고 회차/프레임 id 포함), cue 연결, 필요한 그림을 적는다.
   근거 프레임 크롭은 `art/<이름>/ref/behavior/` 에. 프레임 id 가 없는 동작은 넣지 않는다.
2. `art/<이름>/ref/real/` 에 애니 캡처 크롭, `art/<이름>/prompts/` 에 프롬프트 (`art/usagi/prompts` 를 복사해 생김새 부분만 바꾸되,
   머리 단계 `2x_*.txt` 는 행동 시트의 "머리 단계" 표대로 캐릭터 성격에 맞게 새로 쓴다). 행동 그림은 `6x_behavior_<이름>.txt`.
3. `art/scripts/gen.sh -c <이름> -i <입력> -r <참고>... -n 4 00_base.txt` 로 base 후보, 고른 뒤 나머지 변형과 행동 그림 생성 (`-p` 로 근거 장면 첨부).
4. `art/<이름>/picks.json` 에 고른 후보, `_title`(메뉴 이름), `_scenes`/`_behaviors`, `_timing`/`_anim` 을 적고 `art/scripts/build.sh --name <이름>`.
5. `typing-pet --check characters/<이름>` 통과, `--render-all` 시트와 `--render-timeline` 로 눈으로 확인.

`_test` 는 색 도형으로 만든 검증용 가짜 세트다 (`art/scripts/make_test_layers.py`).
