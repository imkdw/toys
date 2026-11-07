# 동영상 몰아보기 생성기

플레이리스트의 영상에서 오프닝과 엔딩을 잘라내고, 화수 표시와 챕터를 넣어 MP4 1개로 합치는 도구입니다.

작품에 맞춘 값(플레이리스트 URL, 제목 형식, 감지 기준값, 기준 이미지)은 코드에 없고, Git에서 제외된 `work/` 안의 `settings.json`과 `refs/`에서 읽습니다.

## 준비

Node.js, pnpm, ffmpeg, uv가 필요합니다. `yt-dlp`는 `uvx --python 3.12`로 실행합니다.

```bash
pnpm install
pnpm typecheck
```

## 실행 순서

```bash
npx tsx src/episodes.ts            # 목록 확정
npx tsx src/download.ts            # 다운로드 (이어받기 가능)
npx tsx src/detect.ts              # 구간 감지. 표시된 화는 work/overrides.json 에 판정을 쓰고 다시 실행
npx tsx src/sheets.ts cuts         # 컷 경계 검수 시트
npx tsx src/title-card.ts          # 화수 표시 PNG
npx tsx src/encode.ts              # 화별 인코딩. 재판정되지 않은 화는 건너뜀
npx tsx src/verify.ts episodes     # 원본 대조
npx tsx src/concat.ts              # 합치기. 원본 대조를 통과한 화만 포함
npx tsx src/verify.ts final        # 최종 파일 검사
npx tsx src/sheets.ts junctions    # 연결 지점 검수 시트
npx tsx src/report.ts              # work/REPORT.md 생성
```

대부분의 명령은 `--pilot`(설정의 일부 화만) 또는 `--episodes 1,5,9`를 받습니다. `concat`에 `--episodes`를 주면 `final.mp4` 대신 `partial.mp4`를 만듭니다.

## 구조

| 경로 | 역할 |
|---|---|
| `src/config.ts` | 경로와 기본값 |
| `src/episodes.ts` | 목록 확정 |
| `src/download.ts` | 다운로드 |
| `src/detect.ts` | 구간 감지 |
| `src/sheets.ts` | 검수 시트 |
| `src/title-card.ts` | 화수 표시 PNG |
| `src/encode.ts` | 화별 인코딩 |
| `src/concat.ts` | 합치기와 챕터 |
| `src/verify.ts` | 1~4차 검증 |
| `src/report.ts` | 리포트 |
| `work/` | 영상과 중간 산출물 (Git 제외) |
