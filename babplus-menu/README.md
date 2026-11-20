# 코딩머신 (kimdongmu)

Discord 봇 "코딩머신" 입니다. 상시 실행되면서 매분 기능별 작업을 cron 식(KST 기준)에 맞춰 실행하는 작은 스케줄러 위에 기능을 하나씩 얹는 구조입니다.

지금 들어 있는 기능은 밥풀러스 식단 알림 하나입니다. 밥풀러스 카카오 채널(`pf.kakao.com/_hqGbn`)에 전날 저녁 올라오는 월요일/금요일 중식 메뉴 글을 찾아 메뉴판 이미지를 Discord 채널로 보내 줍니다. 외부 의존성 없이 Node.js 내장 기능(fetch / node:test / TypeScript 타입 제거 실행)만 씁니다.

## 주요 기능

- **식단 전송** (`babplus:send`): 일/목 20:05 부터 20:55 까지 10분마다, 내일 날짜의 `M/D (요일) 메뉴` 글을 찾아 메뉴판 이미지(가로 1500px 미만, 최대 10장)를 첨부해 보냅니다. 석식메뉴 글과 음식 사진은 거릅니다.
- **중복 방지**: 보낸 메시지의 embed footer 에 글 ID 표식을 넣고, 보내기 전에 채널 최근 메시지 20개를 확인해 같은 글을 다시 보내지 않습니다.
- **실패 알림** (`babplus:miss`): 일/목 21:05 에 한 번 더 시도하고, 그래도 못 보냈으면 이유(글 없음 / 이미지 없음 / 카카오 조회 실패)를 담은 실패 알림을 하루 한 번 남깁니다.
- **네트워크 재시도**: 연결 단계 오류(연결 시간 초과, DNS 실패 등)는 2초 / 5초 / 10초 간격으로 다시 시도하고, Discord 429 응답은 `retry_after` 만큼 기다렸다 한 번 더 보냅니다.
- **dry-run**: Discord 로 보내지 않고 보낼 내용을 로그로만 남깁니다.

## 요구 사항

- Node.js 22.18 이상 (`.ts` 파일을 그대로 실행합니다)
- Discord 봇 토큰과 알림을 받을 채널 ID

## 설정

저장소 루트의 `.env` 에서 읽습니다. 이미 설정된 환경 변수는 덮어쓰지 않습니다. 예시는 `deploy/.env.example` 에 있습니다.

| 변수 | 설명 |
| --- | --- |
| `DISCORD_BOT_TOKEN` | Discord 봇 토큰. 없으면 `DRY_RUN=1` 이 아닌 한 종료 코드 78 로 끝납니다. |
| `BABPLUS_CHANNEL_ID` | 식단을 보낼 채널 ID |
| `DRY_RUN` | `1` 이면 전송하지 않고 로그만 남깁니다. |
| `NODE_BIN` | `deploy/start.sh` 와 `deploy/deploy.sh` 가 쓸 node 경로. 비우면 `~/.nvm` 에서 가장 높은 버전을 씁니다. |
| `BABPLUS_DAYS` | 대상 요일을 바꿉니다(예: `수`, `월,금`). 수동 실행 시험용이며 기본값은 `월,금` 입니다. |

`.env` 는 git 에 올리지 않습니다.

## 실행

```bash
cp deploy/.env.example .env   # 값 채우기
npm start                     # node src/index.ts, 상시 실행 스케줄러
npm test                      # node --test test/*.test.ts
DRY_RUN=1 npm start           # 토큰 없이 로그만 확인
```

### 작업 하나만 지금 실행

수동 재전송이나 시험에 씁니다. `--now` 로 기준 시각(KST)을 바꿀 수 있고, 식단 작업은 기준 시각의 "내일" 식단을 대상으로 합니다.

```bash
node src/run.ts <작업> [--dry-run] [--now "YYYY-MM-DD HH:mm"]

node src/run.ts babplus:send --dry-run --now "2026-11-15 20:05"
node src/run.ts babplus:miss
```

작업 이름을 빼거나 틀리면 사용법과 작업 목록을 출력합니다.

## 배포

N100 서버에 systemd 사용자 서비스로 올립니다. 맥에서 직접 실행하는 수동 배포입니다.

```bash
./deploy/deploy.sh            # 기본 호스트 n100
HOST=other ./deploy/deploy.sh
```

`deploy.sh` 는 다음을 합니다.

1. rsync 로 `~/kimdongmu` 에 복사 (`.env` / `.git` / `node_modules` / `docs` 제외)
2. 원격에서 테스트 실행
3. `deploy/kimdongmu.service` 를 `~/.config/systemd/user/` 에 설치하고 다시 띄움

원격 `~/kimdongmu/.env` 에 `DISCORD_BOT_TOKEN` 이 없으면 서비스를 띄우지 않고 안내만 출력합니다. 서비스는 `deploy/start.sh` 로 node 를 찾아 실행하며, 종료 코드 78(설정 누락)일 때는 재시작하지 않습니다.

```bash
systemctl --user status kimdongmu.service
journalctl --user -u kimdongmu.service -f
```

## 새 기능 추가

`src/features/<이름>/` 폴더에 `Feature`(`name` 과 `{ id, cron, run }` 작업 목록)를 만들고 `src/features/index.ts` 의 배열에 추가합니다. 작업 키는 `<기능>:<작업 id>` 가 되고, cron 식은 KST 기준 5필드(`*`, `a-b`, `a,b`, `*/n`, `a-b/n` 지원)입니다.

## 디렉터리 구조

```
src/
  index.ts              상시 실행 진입점
  run.ts                작업 하나 수동 실행
  scheduler.ts          cron 해석과 매분 실행
  discord.ts            Discord REST 클라이언트(전송 / 최근 메시지 조회)
  net.ts                연결 오류 재시도 fetch
  env.ts / time.ts      .env 로딩, 로그, KST 시간 계산
  features/
    index.ts            기능 목록
    babplus/            밥풀러스 식단 알림 (kakao.ts: 카카오 채널 글 조회와 판별)
test/                   node:test 테스트와 카카오 응답 fixture
deploy/                 배포 스크립트, systemd 유닛, .env 예시
docs/                   기획 문서(plan.html)와 참고 이미지
```
