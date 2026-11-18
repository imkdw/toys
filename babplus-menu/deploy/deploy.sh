#!/usr/bin/env bash
# 맥에서 직접 실행하는 수동 배포. 파일을 N100 으로 복사하고 테스트를 돌린 뒤 서비스를 다시 띄운다.
#   ./deploy/deploy.sh            # 기본 호스트 n100
#   HOST=other ./deploy/deploy.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOST="${HOST:-n100}"
DEST="kimdongmu"

rsync -a --delete \
  --exclude .env --exclude .git --exclude .DS_Store --exclude node_modules --exclude docs \
  "$ROOT/" "$HOST:$DEST/"

ssh "$HOST" DEST="$DEST" bash -s <<'REMOTE'
set -euo pipefail
cd "$HOME/$DEST"
chmod +x deploy/start.sh deploy/deploy.sh
NODE_BIN=""
if [[ -f .env ]]; then NODE_BIN="$(sed -n 's/^NODE_BIN=//p' .env | tail -1)"; fi
NODE_BIN="${NODE_BIN:-$(ls -d "$HOME"/.nvm/versions/node/v*/bin/node 2>/dev/null | sort -V | tail -1)}"
echo "node: $NODE_BIN ($("$NODE_BIN" -v))"
"$NODE_BIN" --test test/*.test.ts | tail -n 8

mkdir -p "$HOME/.config/systemd/user"
cp deploy/kimdongmu.service "$HOME/.config/systemd/user/kimdongmu.service"
systemctl --user daemon-reload

if [[ ! -f .env ]] || ! grep -q '^DISCORD_BOT_TOKEN=.' .env; then
  echo
  echo "~/$DEST/.env 에 DISCORD_BOT_TOKEN 이 없어 서비스를 띄우지 않았습니다."
  echo "  cp ~/$DEST/deploy/.env.example ~/$DEST/.env && chmod 600 ~/$DEST/.env"
  echo "토큰을 넣은 뒤 deploy.sh 를 다시 실행하세요."
  exit 0
fi
chmod 600 .env
systemctl --user enable kimdongmu.service >/dev/null
systemctl --user restart kimdongmu.service
sleep 2
systemctl --user --no-pager status kimdongmu.service | head -n 5
journalctl --user -u kimdongmu.service -n 5 --no-pager
REMOTE
