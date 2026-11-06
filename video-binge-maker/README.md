# 동영상 몰아보기 생성기

플레이리스트의 영상에서 오프닝과 엔딩을 잘라내고, 화수 표시와 챕터를 넣어 MP4 1개로 합치는 도구입니다.

현재는 프로젝트 구조만 있으며 기능은 아직 구현되지 않았습니다.

## 준비

Node.js, pnpm, ffmpeg, uv가 필요합니다. `yt-dlp`는 `uvx --python 3.12`로 실행합니다.

```bash
pnpm install
pnpm typecheck
```

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
