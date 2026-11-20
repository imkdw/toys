# ascii-art

사진이나 영상을 터미널에서 24비트 컬러 ASCII 아트로 보여주는 Rust CLI. 이미지는 한 장 출력하고, 영상/GIF 는 반복 재생한다. 디코딩은 ffmpeg/ffprobe 에 맡기고, 이 프로그램은 프레임을 글자 격자와 색으로 바꿔 그리는 일만 한다.

캐릭터 그림처럼 배경이 단순한 이미지는 배경을 지우고 외곽선/눈/입을 살려 피사체만 그리고, 배경이 있는 장면은 화면 전체를 채워 그린다.

## 주요 기능

- 이미지(png, jpg, jpeg, webp, bmp, ppm, pgm, tif, tiff) 한 장 출력, 영상/GIF 반복 재생 (`--once` 로 한 번만)
- 터미널 크기와 원본 비율에 맞춘 자동 칸 수 계산 (`--cols` 로 직접 지정 가능)
- 배경 자동 제거: 투명 배경, 단색 배경, 외곽선으로 닫힌 캐릭터 순으로 시도하고 안 되면 화면 전체를 채움
- 피사체 모드: 어두운 선을 따로 그려 외곽선/눈/입을 드러내고, 왼쪽 위에서 비춘 듯한 입체 음영을 입힘
- 장면 모드: 밝기 평활화로 글자를 고르고 명암은 주로 색으로 표현
- 문자 램프 프리셋과 직접 지정, 대비/감마/밝기/반전 조절
- 글자가 위에서 쏟아지며 완성되는 등장 효과 (`--intro`)
- 시간 기준 재생(늦으면 프레임 건너뜀), 한 바퀴 출력을 메모리에 담아 다음 바퀴부터 재사용
- 동기화 출력(DEC 2026)으로 프레임 찢김 방지
- 색 없는 텍스트 한 프레임 출력 (`--snapshot`), 골든 비교 테스트에 사용

## 요구 사항

- Rust (edition 2024 를 지원하는 툴체인)
- ffmpeg, ffprobe (PATH 에 있어야 한다)
- 트루컬러(24비트)를 지원하는 터미널 권장: iTerm2, Ghostty, WezTerm, kitty, 최신 Terminal.app

## 설치

```sh
brew install ffmpeg          # ffmpeg, ffprobe
cargo install --path .       # ~/.cargo/bin/ascii-art 에 설치
```

설치 없이 실행하려면 `cargo run --release -- <파일> [옵션]`.

## 사용법

```sh
ascii-art usagi.png                  # 이미지 한 장 출력 (터미널 크기에 맞춤)
ascii-art clip.mp4                   # 영상/GIF 반복 재생, Ctrl+C 로 종료
ascii-art clip.mp4 --once --info     # 한 번만 재생, 위쪽에 정보 줄(파일명, 경과 시간, 글자 수)
ascii-art usagi.png --intro          # 등장 효과
ascii-art usagi.png --cols 80 --no-color > out.txt
ascii-art clip.mp4 --snapshot --at 3 # 3초 위치 한 프레임을 색 없이 텍스트로
```

### 출력/재생 옵션

| 옵션 | 설명 |
| --- | --- |
| `--cols N` | 가로 칸 수 (기본: 터미널에 맞춤) |
| `--no-color` | 색 없이 글자만 |
| `--char-aspect F` | 글자 세로/가로 비율 (기본 2.0) |
| `--info` | 위쪽에 정보 줄 표시 |
| `--intro` | 등장 효과 |
| `--fps F` | 재생 fps (기본: 원본, 최대 30) |
| `--once` | 한 번만 재생 |
| `--snapshot` | 한 프레임을 색 없이 텍스트로 출력 (`--cols` 없으면 80칸) |
| `--at SEC` | `--snapshot` 에서 뽑을 시각 (기본 0) |

### 화질 옵션

| 옵션 | 설명 |
| --- | --- |
| `--ramp NAME\|문자열` | 문자 램프. `letters`(기본, `n0Q8B&#$@`), `standard`, `dense`, `simple`, `blocks` 또는 직접 지정(어두운 것부터) |
| `--contrast F` / `--gamma F` / `--brightness F` | 대비 / 감마(1 보다 크면 밝아짐) / 밝기 더하기(-1~1) |
| `--invert` | 밝기 반전 (밝은 배경 터미널용) |
| `--bg MODE` | 배경 제거: `auto`(기본), `none`(화면 전체를 채움), `alpha`, `outline`, `#rrggbb` |
| `--bg-tol F` | 배경색 허용 거리 (0~1, 기본 0.12). 배경이 덜 빠지면 올린다 |
| `--shade F` | 입체 음영 세기 (0~1). 기본은 자동: 배경을 지운 그림 1, 배경 있는 장면 0 |
| `--ink-thr F` | 이보다 어두운 화소를 선(외곽선, 눈, 입)으로 본다 (0~1, 기본 0.36). 선이 덜 보이면 올린다 |
| `--ink-gap` | 선을 선 색 글자로 그리지 않고 빈칸으로 비운다 |
| `--no-bold` | 굵은 글씨 끄기 |
| `--edges` | 경계 칸에 방향 글자(`- / \| \\`)를 쓴다 (기본 끔) |
| `--edge-thr F` | `--edges` 판정 세기 (기본 0.12, 낮을수록 많이) |
| `--equalize F` | 밝기 평활화 비율 (0~1). 기본은 자동: 배경을 지운 그림 0, 배경 있는 장면 0.7 |
| `--color-floor F` | 어두운 칸 전경색 최소 밝기 (0~1, 기본 0.35) |

값은 `--cols 80` 과 `--cols=80` 두 형식 모두 받는다. 전체 옵션은 `ascii-art --help`, 버전은 `ascii-art --version`.

글자 크기를 줄일수록 칸이 많아져 그림이 또렷해진다 (예: 8~10pt).

## 동작 방식

1. ffprobe 로 크기/fps/회전 정보를 읽고, 원본 비율과 글자 비율(세로 = 가로 x 2)로 칸 수를 정한다.
2. ffmpeg 가 칸마다 4x8 부분 화소 크기로 줄인 RGBA 프레임을 파이프로 넘긴다.
3. 부분 화소로 배경 마스크를 만든다. 투명 배경은 알파로, 단색 배경은 가장자리에서 시작하는 flood fill 로, 배경 그림이 있는 장면은 어두운 외곽선을 벽으로 보고 가장자리에서 번져 간 곳을 지운다. 셋 다 안 되면 장면 모드로 화면 전체를 채운다.
4. 피사체 모드는 선이 걸친 칸을 선 색 글자(굵게 덮이면 `@`, 살짝 걸치면 `*`)로 그리고, 면은 램프 글자로 채운 뒤 입체 음영을 입힌다. 장면 모드는 밝기를 평활화해 글자를 고른다.
5. 색이 바뀔 때만 색 코드를 쓰고, 프레임은 커서를 처음으로 옮겨 덮어쓴다.

## 환경변수

| 변수 | 설명 |
| --- | --- |
| `ASCII_ART_STATS` | 파일 경로를 주면 재생 통계(표시/건너뜀 프레임, 평균 렌더 ms 등)를 기록한다 |
| `COLUMNS` / `LINES` | 터미널 크기를 읽을 수 없을 때 쓰는 칸/줄 수 (기본 100 / 40) |
| `UPDATE_GOLDEN` | 테스트에서 골든 파일을 현재 출력으로 갱신한다 |

## 개발

```sh
cargo test --release                     # 단위 테스트 + --snapshot 골든 비교 (ffmpeg 가 없으면 골든 비교는 건너뜀)
UPDATE_GOLDEN=1 cargo test --release     # 의도한 화질 변경 후 골든 갱신
ASCII_ART_STATS=stats.txt ascii-art clip.mp4 --once   # 재생 통계 기록
```

## 디렉터리 구조

```
src/
  main.rs     진입점
  cli.rs      인자 해석과 실행, --help 문구
  media.rs    ffprobe/ffmpeg 로 입력을 읽어 RGBA 프레임 생성
  render.rs   프레임을 글자 격자(글자 + 색)로 바꾸고 터미널 문자열로 인코딩
  play.rs     크기 계산, 이미지 출력, 영상 재생, 등장 효과
  term.rs     터미널 크기, 시그널, 화면 상태 복구
tests/
  snapshot.rs 고정 입력에 대한 --snapshot 출력 골든 비교
  golden/     기대 출력
```
