# card-enhance-demo

휴대폰으로 찍은 카드(포토카드, 스티커 카드 등) 사진을 올리면 여백 없이 잘라 정방향으로 세우고 색을 보정한 카드 이미지 URL을 돌려주는 개인용 데모.

이미지를 새로 그리지 않는다. `codex` CLI(구독 로그인)는 카드 네 꼭짓점, 위쪽 방향, 비율, 모서리 둥글기, 색 보정 값만 JSON으로 분석하고, 실제 크롭/원근 보정/색 보정은 코드(sharp)가 한다. 그래서 결과물에는 원본 카드가 그대로 남는다.

## 주요 기능

- 웹 페이지에서 끌어다 놓기나 파일 선택으로 여러 장 업로드, 단계별 진행 상태와 원본/결과 비교, 분석 JSON과 codex 프롬프트 확인
- HTTP API로 작업 생성/조회
- 폴더째 일괄 업로드 스크립트(`pnpm submit`)
- 저장된 분석 결과로 codex 호출 없이 보정만 다시 돌리는 스크립트(`pnpm reprocess`)
- 작업 큐와 동시 실행 수 제한, 서버 재시작 후 이전 작업 목록 복원

## 처리 흐름

| 단계 | 상태값 | 담당 | 내용 |
|---|---|---|---|
| 1. 입력 정리 | `queued` | sharp | EXIF 방향을 반영해 `source.png` 로 변환 |
| 2. 분석 | `analyzing` | codex (`--output-schema`) | 카드 꼭짓점(0~1 정규화 좌표), `cardTopFacing`, `cardAspectRatio`, `cornerRadiusRatio`, 밝기/대비/채도/채널별 배율, 문제점과 개선안 |
| 3. 보정 | `correcting` | sharp | 꼭짓점 기준 원근 보정(정방향 회전 포함), 자동 레벨, 분석 값 기반 색 보정(안전 범위로 제한), 샤픈, 둥근 모서리 마스크로 투명 PNG 출력 |
| 4. 완료 | `done` / `failed` | | 결과 `result.png` |

분석 스키마는 `src/server/schema/analysis.json`, 프롬프트는 `src/server/prompts.ts` 에 있다.

## 요구 사항

- Node.js, pnpm (`packageManager`: pnpm@11.8.0)
- `codex` CLI 설치 및 로그인 (`codex exec` 로 호출)

## 설치와 실행

```bash
pnpm install
pnpm dev      # 웹 빌드(vite build) 후 서버 실행
```

서버는 기본으로 `0.0.0.0:4000` 에서 뜨고, 시작 로그에 로컬 주소와 같은 네트워크에서 접속할 `http://<내 IP>:4000` 주소가 찍힌다.

| 스크립트 | 내용 |
|---|---|
| `pnpm dev` | `vite build` 후 서버 실행 |
| `pnpm start` | 서버만 실행 (이미 빌드된 `dist/web` 제공) |
| `pnpm build:web` | 웹만 빌드 (`dist/web`) |
| `pnpm typecheck` | `tsc --noEmit` |
| `pnpm submit <폴더 또는 파일...> [--server=URL]` | jpg/png/webp 파일을 서버에 업로드. `--server` 기본값 `http://localhost:4000` |
| `pnpm reprocess [작업 id...]` | `done` 상태 작업의 `analysis.json` 으로 `result.png` 를 다시 만듦. id를 생략하면 전체 |

예:

```bash
pnpm submit data/cardform
pnpm submit a.jpg b.png --server=http://192.168.0.21:4000
```

## API

| 메서드 | 경로 | 설명 |
|---|---|---|
| POST | `/api/jobs` | multipart `image` 필드 (jpg/png/webp, 15MB 이하). 202와 작업 정보 |
| GET | `/api/jobs/:id` | 작업 상태. `status` 가 `done` 이면 `imageUrl` 에 결과 URL |
| GET | `/api/jobs` | 전체 작업 목록 (최신순) |
| GET | `/files/:id/<파일>` | 작업 폴더의 파일 |

응답 예:

```json
{
  "id": "4af5a9c0",
  "name": "card.jpg",
  "status": "done",
  "createdAt": "...",
  "finishedAt": "...",
  "originalUrl": "http://192.168.0.21:4000/files/4af5a9c0/input.jpg",
  "imageUrl": "http://192.168.0.21:4000/files/4af5a9c0/result.png",
  "analysis": { },
  "prompt": "...",
  "error": null
}
```

## 작업 파일

작업마다 `data/jobs/<id>/` 에 남는다.

- `input.<ext>`: 업로드 원본
- `source.png`: 방향 정리한 입력
- `prompt.txt`, `analysis.json`, `analysis.log`: codex 프롬프트/분석 결과/실행 로그
- `result.png`: 결과 이미지
- `job.json`: 작업 상태 (서버 시작 시 다시 읽음. 진행 중이던 작업은 `failed` 로 처리)

codex는 작업 폴더를 작업 디렉터리로 쓰기 때문에 꼭짓점 확인용 중간 이미지가 같이 남을 수 있다.

## 환경 변수

| 이름 | 기본값 | 설명 |
|---|---|---|
| `PORT` | `4000` | 서버 포트 |
| `HOST` | `0.0.0.0` | 바인드 주소 |
| `DATA_DIR` | `data/jobs` | 작업 폴더 위치 |
| `CODEX_BIN` | `codex` | codex 실행 파일 |
| `CODEX_TIMEOUT_MS` | `720000` (12분) | codex 호출 1회 제한 시간 |
| `CONCURRENCY` | `2` | 동시에 처리하는 작업 수 (구독 사용량 한도 고려) |

## 디렉터리 구조

```
src/server/
  index.ts        Hono 서버, API 라우트, 정적 파일
  jobs.ts         작업 생성/저장/복원, 큐
  pipeline.ts     작업 처리 흐름
  codex.ts        codex exec 호출
  prompts.ts      분석 프롬프트
  image.ts        원근 보정, 자동 레벨, 색 보정, 모서리 마스크
  image-type.ts   업로드 형식 판별
  config.ts       환경 변수
  schema/analysis.json  분석 결과 JSON 스키마
src/shared/types.ts     서버/웹 공용 타입
web/                    업로드 페이지 (vite, 빌드 결과는 dist/web)
scripts/                submit.ts, reprocess.ts
data/                   작업 데이터 (git 제외)
```
