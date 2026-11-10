# characters

캐릭터 하나당 폴더 하나. `_` 로 시작하는 폴더(`_test`)만 git 에 올리고, 나머지는 올리지 않는다.

```
characters/<name>/
├── manifest.toml   메뉴 이름(title), 레이어 목록, 마스크, 발끝 좌표, 머리 기준점, 마우스 연출 축([swing])
│                   애니 장면([[scene]])
│                   ([timing] [anim] [keys] 로 기본값 덮어쓰기)
├── layers/
│   ├── base.png             책상과 소품까지 포함한 기본 장면
│   ├── head/*.png           머리 변형 (마스크 안을 교체)
│   ├── paw_l/*.png          왼앞발 자세
│   ├── paw_r/*.png          오른앞발 자세 (mouse_bg: 마우스 자세에서 팔을 치운 바탕)
│   ├── fx/*.png             선글라스 단독, 마우스 잡은 팔(mouse_arm) (일반 합성)
│   ├── full/*.png           장면 프레임(full/<장면>_a, _b ..)과 제스처 같은 전체 프레임
│   └── mask/*.png           head, paw_l, paw_r 영역 마스크
└── baseline/       조합 시트 기준 이미지 (scripts/verify.sh)
```

모든 레이어는 같은 크기의 정사각형 캔버스다. `art/scripts/build.sh` 가 생성 원본에서 만든다.
앞발/제스처 레이어는 base 와 실제로 달라진 곳만 변형 그림이고 나머지(책상, 키보드)는 base 와 같다.
`paw_r/mouse_bg` + `fx/mouse_arm` + `[swing]` 이 있으면 마우스를 움직이는 동안 팔만 root 에서 tip 쪽으로 휘며 흔든다.
없으면 `paw_r/mouse` 한 장으로 가만히 있다.
메뉴바의 **캐릭터** 메뉴에 `_` 로 시작하지 않는 폴더가 manifest 의 `title`(없으면 폴더 이름) 순으로 나오고, 고르면 바로 바뀌며 설정에 저장된다.
설정에서 캐릭터를 지정하지 않았거나 지정한 폴더가 없으면 그 목록의 첫 번째(지금은 우사기)를 쓴다.

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

| 캐릭터 | idle | milestone | gesture (과열이 식을 때) |
|---|---|---|---|
| 우사기 | 주먹밥을 입 크게 벌려 먹기 (e085, e084) | "야하!" 양팔 번쩍 (e204) | 해골 지팡이 휘두르기 (e036) |
| 치이카와 | 브로콜리 먹고 감동해서 눈물 (e004) | 제초 검정 5급 합격증 들어 올리기 (e224) | 엉엉 울기 (e224, e185) |
| 하치와레 | 코스 요리 책 읽기 (e201) | 카메라로 사진 찍기 (e037, e216) | 기타 치며 노래 |

괄호는 참고한 애니 회차 (`art/<이름>/ref/scene/` 에 크롭). 만드는 법:

1. `art/<이름>/prompts/4x_scene_<장면>_a.txt` (첫 프레임, base 에서 편집), `_b.txt` (둘째 프레임, 고른 첫 프레임에서 편집).
2. `art/scripts/gen.sh -c <이름> -i art/<이름>/raw/base_final.png -r <생김새 참고> -p art/<이름>/ref/scene/<장면>.png -n 2 4x_scene_<장면>_a.txt`
   (`-p` 는 따라 할 장면 캡처. 둘째 프레임은 `-i` 에 고른 첫 프레임을 준다.)
3. `picks.json` 에 `"full/<장면>_a": "<후보>"` 와 `"_scenes": {"<장면>": {"trigger": "idle", "frame_ms": 450}}` 를 적고 build.sh.

새 캐릭터 추가 순서 (생성 원본, 프롬프트, 마스크는 `art/<이름>/` 에 캐릭터별로 둔다):

1. `art/<이름>/ref/real/` 에 애니 캡처 크롭, `art/<이름>/prompts/` 에 프롬프트 (`art/usagi/prompts` 를 복사해 생김새 부분만 바꾼다).
2. `art/scripts/gen.sh -c <이름> -i <입력> -r <참고>... -n 4 00_base.txt` 로 base 후보, 고른 뒤 나머지 변형 생성.
3. `art/<이름>/picks.json` 에 고른 후보와 `_title`(메뉴 이름)을 적고 `art/scripts/build.sh --name <이름>`.
`_test` 는 색 도형으로 만든 검증용 가짜 세트다 (`art/scripts/make_test_layers.py`).
