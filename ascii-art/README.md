# ascii-art

사진이나 영상을 터미널에서 컬러 ASCII 아트로 보여주는 CLI. Rust 단일 바이너리이고, 디코딩은 ffmpeg 에 맡긴다.

## 설치

```sh
brew install ffmpeg          # ffmpeg, ffprobe 필요
cargo install --path .       # ~/.cargo/bin/ascii-art
```

## 사용법

```sh
ascii-art usagi.png                  # 이미지 한 장 출력 (터미널 크기에 맞춤)
ascii-art clip.mp4                   # 영상/GIF 반복 재생, Ctrl+C 로 종료
ascii-art clip.mp4 --once --info     # 한 번만, 위에 정보 줄
ascii-art usagi.png --intro          # 글자가 위에서 쏟아지며 완성되는 등장 효과
ascii-art usagi.png --cols 80 --no-color > out.txt
```

자주 쓰는 화질 옵션

| 옵션 | 설명 |
| --- | --- |
| `--ramp dense` | 문자 램프. `letters`(기본, `n0Q8B&#$@`), `standard`, `dense`, `simple`, `blocks`, 직접 지정(어두운 것부터) |
| `--contrast 1.3`, `--gamma 1.2`, `--brightness 0.1` | 대비/감마/밝기 |
| `--equalize 0.7` | 밝기 평활화. 기본은 자동(배경을 지운 그림 0, 배경 있는 장면 0.7). 밝은 장면이 `@` 로 뭉치는 걸 막는다 |
| `--invert` | 밝기 반전 (밝은 배경 터미널용) |
| `--bg auto` | 배경 제거. 투명 배경, 단색 배경, 외곽선으로 닫힌 캐릭터 순으로 시도하고 안 되면 화면 전체를 채운다. `none`, `alpha`, `outline`, `#00ff00` |
| `--bg-tol 0.2` | 배경색 허용 거리. 배경이 덜 빠지면 올린다 |
| `--shade 0.6` | 입체 음영 세기. 배경을 지운 그림은 기본 1 (왼쪽 위에서 비춘 것처럼 글자가 옅어진다), 0 이면 끔 |
| `--ink-thr 0.36` | 이보다 어두운 화소를 선(외곽선, 눈, 입)으로 본다. 선이 덜 보이면 올린다 |
| `--ink-gap` | 선을 갈색 글자로 그리지 않고 빈칸으로 비운다 |
| `--no-bold` | 굵은 글씨 끄기 |
| `--edges` | 경계 칸에 방향 글자(`- / \| \\`)를 쓴다. 기본은 끔 |

전체 옵션은 `ascii-art --help`.

## 추천 터미널 설정

- 트루컬러(24비트) 터미널: iTerm2, Ghostty, WezTerm, kitty, 최신 Terminal.app
- 글자 크기를 줄일수록 칸이 많아져 그림이 또렷해진다 (예: 8~10pt)
- 동기화 출력(DEC 2026)을 지원하는 터미널(iTerm2, Ghostty, WezTerm, kitty)은 프레임이 찢기지 않는다

## 동작 방식

1. ffprobe 로 크기/fps 를 읽고, 원본 비율과 글자 비율(세로 = 가로 x 2)로 칸 수를 정한다.
2. ffmpeg 가 칸마다 4x8 부분 화소 크기로 줄인 RGBA 프레임을 파이프로 넘긴다.
3. 부분 화소로 배경 마스크를 만든다. 투명 배경은 알파로, 단색 배경은 가장자리에서 시작하는 flood fill 로 지운다. 배경 그림이 있는 애니 장면은 어두운 외곽선을 벽으로 보고 가장자리에서 번져 간 곳을 배경으로 지운다(획 사이 틈은 메우고, 면이 없는 효과선 조각은 버린다). 셋 다 안 되면 장면 모드로 화면 전체를 채운다.
4. 피사체 모드: 어두운 선이 1/3 넘게 걸친 칸은 선 색을 밝힌 글자(굵게 덮이면 `@`, 살짝 걸치면 `*`)로 그려 외곽선과 눈/입이 드러나게 하고(`--ink-gap` 이면 빈칸), 면은 촘촘한 글자(`n0Q8B&#$@`)로 채운다. 실루엣을 여러 크기로 흐려 얻은 기울기를 법선 삼아 왼쪽 위 조명의 입체 음영을 입힌다.
5. 장면 모드: 밝기를 평활화해 글자를 고르고, 명암은 주로 색이 맡는다.
6. 색이 바뀔 때만 색 코드를 쓰고, 프레임은 커서를 처음으로 옮겨 덮어쓴다. 재생은 시간 기준이고 늦으면 프레임을 건너뛴다. 한 바퀴 출력은 메모리에 담아 두고 다음 바퀴부터 재사용한다.

## 개발

```sh
cargo test --release                     # 단위 테스트 + --snapshot 골든 비교
UPDATE_GOLDEN=1 cargo test --release     # 의도한 화질 변경 후 골든 갱신
ascii-art clip.mp4 --snapshot --at 3     # 3초 위치 한 프레임을 색 없이 텍스트로
ASCII_ART_STATS=/tmp/s.txt ascii-art clip.mp4 --once   # 재생 통계(표시/건너뜀 프레임, 렌더 ms)
```
