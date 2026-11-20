# typing-pet

키보드를 치면 바탕화면 위의 캐릭터가 따라서 타이핑하는 macOS 오버레이 앱이다.
투명한 항상 위 창에 책상 앞에 앉은 캐릭터를 그리고, 전역 키보드/마우스 입력을 받아 앞발과 표정을 실시간으로 바꾼다.
메뉴바 아이콘으로 캐릭터, 직군(책상 장비), 표시 옵션을 고른다.

Rust(winit + wgpu) 로 만든 앱 본체와, 생성 이미지를 캐릭터 레이어로 가공하는 Python 스크립트(`art/scripts`)로 이루어져 있다.

## 주요 기능

- **타이핑 따라 하기**: 왼쪽 영역 키는 왼앞발, 오른쪽 영역 키는 오른앞발이 누른다. 마우스를 움직이면 오른앞발이 마우스를 잡고 커서 방향을 따라 움직이며, 클릭하면 살짝 눌린다.
- **표정 단계**: 타이핑 활동 점수에 따라 머리가 normal / excited / dizzy / overheat 로 바뀌고, 오래 쉬면 sleepy 가 된다. 선글라스 이스터에그가 있다.
- **캐릭터 고유 행동**: 캐릭터마다 manifest 의 `[[behavior]]` 로 idle / milestone / gesture / 표정 단계 변화 / 몰아치기 / 특정 키 / 무작위 / 소품 클릭 등에 반응한다.
- **책상 소품과 직군**: 직군(기본 / 개발자 / 디자이너 / 기획자 / 마케터 / 재무/총무 / 인프라 / 보안)마다 책상 위 소품이 바뀌고, 소품을 누르면 소품과 캐릭터가 반응한다.
- **입력 키 보기**: 누른 키 이름이 머리 위에 구름 말풍선으로 떴다 사라진다 (키 값은 저장하지 않는다).
- **창 조작**: 캐릭터를 끌어 옮기고, 캐릭터 위에서 스크롤/핀치로 크기를 바꾼다 (80~800pt). 캐릭터가 아닌 투명한 곳의 클릭은 뒤 앱으로 그대로 간다.
- **오프스크린 렌더 / 검사 CLI**: 창 없이 상태/행동/시나리오를 PNG 로 렌더하고 캐릭터 manifest 규칙을 검사한다.

## 요구 사항

- macOS 13 이상
- Rust (edition 2024 를 빌드할 수 있는 툴체인)
- 실행 시 macOS **입력 모니터링** 권한
- 캐릭터 에셋: `characters/` 아래 캐릭터 폴더. 저장소에는 검증용 가짜 세트 `characters/_test` 만 들어 있고 실제 캐릭터와 `characters/_props` 는 git 에 올리지 않는다.
- 에셋 가공/검증 스크립트용 Python 3 (`art/scripts/requirements.txt`: pillow / numpy / opencv-python-headless). `art/.venv` 가 있으면 그 인터프리터를 쓴다.

## 설치와 실행

```bash
# 개발 실행 (소스 트리의 characters/ 를 읽는다)
cargo run --release

# .app 번들 만들기 -> target/bundle/TypingPet.app
scripts/bundle.sh --make-cert   # 처음 한 번: 서명용 자체 서명 인증서를 로그인 키체인에 만든다
scripts/bundle.sh --install     # 릴리스 빌드, 서명 후 ~/Applications 에 복사

# 다른 맥에 설치할 DMG (universal 바이너리) -> target/dist/TypingPet-<버전>.dmg
scripts/installer.sh
```

같은 인증서로 계속 서명해야 다시 빌드해도 입력 모니터링 권한이 유지된다. 인증서가 없으면 임시 서명되어 빌드마다 권한을 다시 요구한다.

`scripts/bundle.sh` 옵션: `--make-cert`, `--install`, `--debug` (디버그 빌드, `target/bundle-debug/`), `--universal` (Apple Silicon + Intel).

처음 실행하면 입력 모니터링 권한을 요청한다. 권한이 없으면 메뉴바에 경고 항목이 뜨고, 권한이 생기면 자동으로 입력 감지를 시작한다.
.app 으로 실행하면 로그는 `~/Library/Logs/typing-pet.log` 에 남는다.

## 사용법

### 메뉴바

| 항목 | 동작 |
|---|---|
| 캐릭터 | 캐릭터 고르기 (`_` 로 시작하지 않고 manifest.toml 이 있는 폴더) |
| 직군 (책상 장비) | 책상 소품 묶음 고르기 |
| 보이기 / 숨쉬기 / 반응 줄이기 / 입력 키 보기 | 표시 옵션 켜기/끄기 (반응 줄이기: 장면 밖 행동을 끈다) |
| 크기 조절 | 손잡이로 크기/위치를 바꾸는 틀. Enter 또는 두 번 클릭으로 완료, Esc 취소 |
| 로그인 시 자동 실행 | .app 번들로 실행할 때만 켤 수 있다 |
| 입력 모니터링 권한 필요 | 권한이 없을 때만 보이며 시스템 설정을 연다 |

### 명령줄

```bash
typing-pet                                   # 앱 실행
typing-pet --login-item on|off               # 로그인 시 자동 실행 켜기/끄기 (.app 번들에서만)
typing-pet --check characters/<이름>          # 캐릭터 manifest 와 고유 행동 규칙 검사

# 오프스크린 렌더 (공통: --character <이름> 또는 --character-dir <폴더>, --job <id>, --size <px, 기본 480>)
typing-pet --render-state '<상태 JSON>' --out <png> [--wiggle]
typing-pet --render-all <폴더>                # 모든 조합 시트 + behavior 마다 bhv-<이름>.png
typing-pet --render-behaviors <폴더> [--fps n] [--pairs]
typing-pet --render-timeline <폴더> --scenario <이름> [--fps n] [--secs n] [--start 초] [--keys] [--pairs]
```

`--scenario`: demo / idle / milestone / wake / burst / levels / ambient / keys / mouse. 결과 폴더에 프레임 PNG 와 `timeline.json` 이 생긴다.

### 검증 스크립트

| 명령 | 내용 |
|---|---|
| `scripts/verify.sh` | cargo test + 오프스크린 조합 시트 + 기준 이미지 비교. 결과 `target/verify/report.md` |
| `scripts/verify.sh --live [--bundle]` | 위 + probe 로 실제 입력을 보내고 창 캡처, 부드러움/CPU/메모리 측정 |
| `scripts/verify.sh --update-baseline` | 기준 이미지를 지금 렌더 결과로 갱신 |
| `scripts/motion_qa.sh [캐릭터...]` | 움직임 품질 자동 판정. 결과 `target/motion_qa/report.md` |
| `scripts/demo_record.sh` | 시연 인스턴스를 띄워 동작 영상 녹화. 결과 `target/demo/` |
| `cargo test` | 단위/행동/소품/키 말풍선/무작위 입력 테스트 (`tests/`) |

`probe` 바이너리(`cargo run --bin probe -- <all-keys|modifiers|repeat|mouse|burst> --trace <파일>`)는 실제 키/마우스 이벤트를 보내고 앱 트레이스로 반응을 확인하는 검증 도구다. `--live` 검증이 사용한다.

## 설정

설정 파일: `~/Library/Application Support/typing-pet/config.toml` (메뉴에서 바꾸면 자동 저장). 파일이 없거나 깨졌으면 기본값을 쓴다.

| 키 | 기본값 | 설명 |
|---|---|---|
| `character` | 없음 (메뉴 이름순 첫 캐릭터) | `characters/` 아래 폴더 이름 |
| `position` | 없음 (주 화면 우하단) | 캐릭터 좌상단 논리 좌표 |
| `size` | 240 | 표시 크기 (pt, 80~800) |
| `visible` / `breathing` / `calm` / `key_echo` | true / true / false / false | 메뉴 옵션 |
| `job` | `basic` | 직군 id (`characters/_props/props.toml` 의 `[[job]]`) |

환경 변수:

| 이름 | 용도 |
|---|---|
| `TYPING_PET_CONFIG` | 설정 파일 경로 바꾸기 |
| `TYPING_PET_CHARACTERS` | 캐릭터 폴더 루트 바꾸기 (기본: .app 의 Resources, 없으면 소스 트리 `characters/`) |
| `TYPING_PET_TRACE` | 입력/상태/그린 레이어를 JSON 줄로 기록할 파일 (디버그 빌드 전용) |
| `TYPING_PET_NO_INPUT` | 입력 감지 끄기 (CPU 측정용) |
| `TYPING_PET_BENCH` | 지정한 초만큼 계속 그리는 벤치마크 |
| `TYPING_PET_FRAME_LATENCY` | surface 최대 프레임 지연 (기본 2) |
| `RUST_LOG` | 로그 수준 (기본 info) |

## 캐릭터 에셋

캐릭터는 폴더 하나에 `manifest.toml` 과 같은 크기 정사각형 레이어 PNG(`base`, `head/`, `paw_l/`, `paw_r/`, `fx/`, `full/`, `mask/`)로 이루어진다.
생성 원본에서 레이어를 만드는 과정은 `art/scripts/build.sh --name <이름>` 이 한 번에 돌리고, 이미지 생성은 `art/scripts/gen.sh` 를 쓴다.
manifest 형식, `[[behavior]]` cue 와 motion/fx 종류, 새 캐릭터 규칙과 추가 순서는 [characters/README.md](characters/README.md) 에 있다.

## 디렉터리 구조

```
src/
├── main.rs        이벤트 루프, 창, 메뉴 동작, 오프스크린 렌더/검사 CLI
├── input.rs       전역 키보드/마우스 감지 (입력 모니터링 권한)
├── pet.rs         상태 머신: 앞발, 머리 단계, 활동 점수, cue
├── anim.rs        연출: 장면, behavior, motion/fx, 숨쉬기
├── render.rs      wgpu 렌더러, 히트맵, 오프스크린 렌더
├── config.rs      설정 파일, manifest 읽기와 검사
├── desk.rs        책상 소품 / 직군
├── keyecho.rs     입력 키 말풍선
├── tray.rs        메뉴바
├── mac.rs         macOS 창 설정, 로그인 항목
└── bin/probe.rs   실제 입력을 보내는 검증 도구
tests/             통합 테스트와 기준 이미지 (tests/baseline)
characters/        캐릭터 manifest 와 레이어 (_test 만 저장소에 포함)
art/               캐릭터별 행동 시트(BEHAVIORS.md), picks.json, 에셋 가공 스크립트(art/scripts)
scripts/           번들/DMG, 검증, 움직임 QA, 데모 녹화 스크립트
```
