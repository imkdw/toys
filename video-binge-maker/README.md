# 동영상 몰아보기 생성기

플레이리스트의 영상에서 오프닝과 엔딩을 프레임 단위로 잘라내고, 화면 위쪽에 화수 표시를 얹은 뒤 화마다 챕터를 넣어 MP4 1개로 합치는 TypeScript 스크립트 모음입니다.

작품에 맞춘 값(플레이리스트 URL, 제목 형식, 감지 기준값, 기준 이미지)은 코드에 없고, Git에서 제외된 `work/` 안의 `settings.json` 과 `refs/` 에서 읽습니다. 본편이 잘리지 않는 것을 우선해서, 판단하지 못한 쪽은 자르지 않고 표시만 남깁니다.

## 주요 기능

- yt-dlp 로 플레이리스트를 읽어 제목 정규식으로 본편만 골라 화수 순으로 정리
- 1080p 원본 다운로드, 이어받기와 재시도
- 장면 전환/로고 상관계수/크레딧 기준 이미지/음량으로 오프닝 끝과 엔딩 시작을 감지하고, `overrides.json` 으로 화별 수동 판정 반영
- 컷 경계/화수 표시/연결 지점을 한눈에 보는 검수 시트(JPG) 생성
- 화별 H.264 인코딩과 PCM 음성 추출. 음성은 합칠 때 전체를 AAC 로 한 번만 인코딩해 싱크가 누적해서 틀어지지 않게 함
- 원본 대조(SSIM/음성 상호상관)와 최종 파일 검사(디코드/PTS/챕터/싱크)를 통과한 화만 최종 파일에 포함
- 제외/수동 판정/자르지 않은 화/실패한 검사를 모은 `work/REPORT.md` 생성

## 요구 사항

- macOS (화수 표시 PNG 가 시스템 글꼴 Apple SD Gothic Neo 를 씀)
- Node.js, pnpm
- ffmpeg, ffprobe
- uv (`yt-dlp` 를 `uvx --python 3.12` 로 실행)
- bun (다운로드할 때 yt-dlp 의 JS 런타임으로 씀)

## 설치

```bash
pnpm install
pnpm typecheck
```

실행 전에 `work/settings.json` 을 준비해야 합니다(아래 설정 참고).

## 실행 순서

```bash
npx tsx src/episodes.ts            # 목록 확정 -> work/episodes.json
npx tsx src/download.ts            # 원본 다운로드 -> work/sources/ (이어받기 가능)
npx tsx src/detect.ts --build-refs # 크레딧 기준 이미지 생성 -> work/refs/ (처음 한 번)
npx tsx src/detect.ts              # 구간 감지 -> work/segments.json
npx tsx src/sheets.ts cuts         # 컷 경계 검수 시트
npx tsx src/title-card.ts          # 화수 표시 PNG -> work/title-cards/
npx tsx src/encode.ts              # 화별 인코딩 -> work/encoded/
npx tsx src/sheets.ts titles       # 화수 표시 검수 시트
npx tsx src/verify.ts episodes     # 원본 대조 -> work/results/verify-episodes.json
npx tsx src/concat.ts              # 합치기 -> work/final/final.mp4
npx tsx src/verify.ts final        # 최종 파일 검사 -> work/results/verify-final.json
npx tsx src/sheets.ts junctions    # 연결 지점 검수 시트
npx tsx src/report.ts              # work/REPORT.md 생성
```

- `detect` 가 표시(flags)를 남긴 화는 `work/overrides.json` 에 판정을 쓰고 `detect` 를 다시 실행합니다. `encode` 는 재판정되지 않은 화를 건너뛰고, 입력이 같은 화는 다시 인코딩하지 않습니다.
- `concat` 은 인코딩 결과가 현재 구간과 같고 원본 대조를 통과한 화만 넣습니다.
- 검수 시트는 `work/sheets/` 에 만들어집니다.

## 옵션

| 옵션 | 적용 대상 | 설명 |
|---|---|---|
| `--pilot` | download / detect / sheets / title-card / encode / verify / concat | `settings.json` 의 `pilotEpisodes` 만 처리 |
| `--episodes 1,5,9` | download / detect / sheets / title-card / encode / verify episodes / concat | 지정한 화만 처리 |
| `--refresh` | episodes | 저장된 `work/playlist.raw.json` 대신 플레이리스트를 다시 받음 |
| `--build-refs` | detect | `detect.refs.seeds` 로 크레딧 기준 이미지 생성 |

`concat` 에 `--pilot` 을 주면 `pilot.mp4`, `--episodes` 를 주면 `partial.mp4` 를 만들어 `final.mp4` 를 덮어쓰지 않습니다. 챕터 정보는 같은 이름의 `.chapters.json` 으로 함께 저장됩니다.

## 설정

`work/settings.json` 의 주요 키:

| 키 | 내용 |
|---|---|
| `playlistUrl` | 대상 플레이리스트 URL |
| `titlePattern` | 제목에서 화수(1번 그룹)와 부제(2번 그룹)를 뽑는 정규식 |
| `expectedEpisodeCount` / `excludedEpisodes` | 목록 검사용 전체 화 수와 비어 있어도 되는 화수 |
| `pilotEpisodes` | `--pilot` 에 쓰는 화수 |
| `detect` | 구간 감지 기준값(`analysis` / `spec` / `opening` / `credits` / `afterCredits` / `main` / `audio` / `refs`). 빠진 값이 있으면 시작 전에 멈춤 |
| `encode` | `crf` / `preset` (기본 18 / slow) |
| `verify` | 검증 기준값 덮어쓰기(선택). 기본값은 `src/verify.ts` 의 `DEFAULTS` |

`work/overrides.json` 은 화수를 키로 `startFrame` / `endFrame` / `uncutOpening` / `uncutEnding` / `exclude` 와 `reason`(필수)을 적는 수동 판정 파일입니다. `work/report-notes.md` 가 있으면 리포트 끝에 그대로 덧붙습니다.

환경변수 `VBM_WORK` 를 주면 `work/` 대신 그 디렉터리를 작업 폴더로 씁니다.

## 구조

| 경로 | 역할 |
|---|---|
| `src/config.ts` | 경로, 설정 로드, 단계 사이 데이터 형식, 화 선택 옵션 처리 |
| `src/episodes.ts` | 목록 확정 |
| `src/download.ts` | 다운로드 |
| `src/detect.ts` | 구간 감지와 기준 이미지 생성 |
| `src/sheets.ts` | 검수 시트 |
| `src/title-card.ts` | 화수 표시 PNG |
| `src/encode.ts` | 화별 인코딩 |
| `src/verify.ts` | 원본 대조와 최종 파일 검사 |
| `src/concat.ts` | 합치기와 챕터 |
| `src/report.ts` | 리포트 |
| `work/` | 설정, 영상, 중간 산출물 (Git 제외) |
