#!/usr/bin/env bash
# systemd 진입점. NODE_BIN 을 정하고 코딩머신 봇을 띄운다. 나머지 설정은 src/env.ts 가 .env 에서 읽는다.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
if [[ -z "${NODE_BIN:-}" && -f .env ]]; then
  NODE_BIN="$(sed -n 's/^NODE_BIN=//p' .env | tail -1)"
fi
if [[ -z "${NODE_BIN:-}" ]]; then
  # nvm 으로 깐 Node 중 가장 높은 버전
  NODE_BIN="$(ls -d "$HOME"/.nvm/versions/node/v*/bin/node 2>/dev/null | sort -V | tail -1 || true)"
fi
NODE_BIN="${NODE_BIN:-$(command -v node || true)}"
if [[ -z "$NODE_BIN" ]]; then
  echo "node 를 찾지 못했습니다. .env 에 NODE_BIN 절대 경로를 넣으세요." >&2
  exit 78
fi
exec "$NODE_BIN" "$@" src/index.ts
