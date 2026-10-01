# card-enhance-demo

카드 사진을 올리면 codex(구독 로그인)로 형태 분석 → 개선안 작성 → 화질업 + 정방향 크롭을 하고, 여백 없는 카드 이미지 URL을 돌려주는 개인용 데모.

## 실행

```bash
pnpm install
pnpm dev          # 웹 빌드 + 서버 실행 (0.0.0.0:4000)
```

같은 네트워크에서는 서버 시작 로그에 찍히는 `http://<내 IP>:4000` 으로 접속한다. `codex` CLI가 설치되어 있고 로그인된 상태여야 한다.

폴더째 올리기:

```bash
pnpm submit data/cardform
```

## 처리 흐름

| 단계 | 담당 | 내용 |
|---|---|---|
| 1. 형태 분석 | codex (`--output-schema`) | 인물/캐릭터 목록, 인쇄 글자, 화질 문제, 개선안, 카드 위쪽 방향, 카드 비율, 모서리 모양 |
| 2. 정방향 회전 | 코드 (sharp) | 분석한 `cardTopFacing` 으로 회전. 모델에 맡기지 않는다 |
| 3. 화질업 + 크롭 | codex (image_generation) | 분석 결과로 만든 이미지별 프롬프트. 카드 밖은 마젠타로 채우게 함 |
| 4. 여백 제거 | 코드 (sharp) | 마젠타를 투명 처리하고 trim. 테두리 없는 PNG |

작업 파일은 `data/jobs/<id>/` 에 남는다 (`analysis.json`, `prompt.txt`, `*.log`, `result.png`).

## API

| 메서드 | 경로 | 설명 |
|---|---|---|
| POST | `/api/jobs` | multipart `image` 필드 (jpg/png/webp, 15MB). 202 + 작업 정보 |
| GET | `/api/jobs/:id` | 작업 상태. `status: done` 이면 `imageUrl` 에 결과 URL |
| GET | `/api/jobs` | 전체 작업 목록 |

```json
{ "id": "4af5a9c0", "status": "done", "imageUrl": "http://192.168.0.21:4000/files/4af5a9c0/result.png", "originalUrl": "...", "analysis": { }, "prompt": "..." }
```

## 환경 변수

| 이름 | 기본값 | 설명 |
|---|---|---|
| `PORT` | 4000 | |
| `HOST` | 0.0.0.0 | |
| `CONCURRENCY` | 2 | 동시에 돌리는 작업 수 (구독 사용량 한도 고려) |
| `CODEX_TIMEOUT_MS` | 360000 | codex 호출 1회 제한 시간 |
