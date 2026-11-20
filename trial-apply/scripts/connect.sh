#!/usr/bin/env bash
#
# connect.sh: 무선 디버깅으로 페어링된 안드로이드 기기를 찾아서 adb 로 붙는다.
#
# 무선 디버깅 포트는 켤 때마다 바뀌고, 기기 이름(시리얼)도 기기마다 다르다.
# 그래서 아무것도 하드코딩하지 않고 mDNS(_adb-tls-connect._tcp)로 매번 찾는다.
#
# 순서:
#   1) 이미 무선으로 붙은 기기가 있으면 그대로 쓴다
#   2) 직전에 성공한 주소(캐시)로 다시 붙어 본다
#   3) mDNS 로 기기를 찾는다 (macOS 는 dns-sd, 그 밖에는 adb mdns services)
#   4) 붙은 주소를 stdout 에 한 줄로 출력한다 (ADB_SERIAL=$(scripts/connect.sh) 처럼 쓰면 된다)
#
# 사용법:
#   scripts/connect.sh                    연결
#   scripts/connect.sh pair <ip:port> <code>   최초 1회 페어링
#   scripts/connect.sh off                연결 해제
#
set -euo pipefail

MDNS_SERVICE='_adb-tls-connect._tcp'
CACHE_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/trial-apply/endpoint"
TRIES=40
INTERVAL=0.25

info() { printf '[*] %s\n' "$*" >&2; }
warn() { printf '[!] %s\n' "$*" >&2; }
die() { printf '[x] %s\n' "$*" >&2; exit 1; }

command -v adb >/dev/null 2>&1 || die "adb 를 찾을 수 없다 (Android SDK platform-tools 를 PATH 에 추가)"

case "${1:-}" in
  pair)
    [ $# -eq 3 ] || die "사용법: scripts/connect.sh pair <ip:port> <code>"
    adb pair "$2" "$3"
    exit 0
    ;;
  off)
    adb disconnect >/dev/null 2>&1 || true
    rm -f "$CACHE_FILE"
    info "연결 해제"
    exit 0
    ;;
esac

live_endpoint() {
  adb devices 2>/dev/null | awk '$2 == "device" && $1 ~ /:/ { print $1; exit }'
}

try_connect() {
  [ -n "$1" ] || return 1
  adb connect "$1" >/dev/null 2>&1 || true
  [ "$(adb -s "$1" get-state 2>/dev/null | tr -d '[:space:]')" = 'device' ]
}

# 파일에서 패턴이 나올 때까지 기다린다. 나오면 그 값을 출력한다.
wait_for() {
  local file="$1" cmd="$2" out i
  for ((i = 0; i < TRIES; i++)); do
    out="$(eval "$cmd" <"$file" 2>/dev/null || true)"
    if [ -n "$out" ]; then printf '%s\n' "$out"; return 0; fi
    sleep "$INTERVAL"
  done
  return 1
}

# macOS: adb 자체 mDNS 는 자주 실패해서 Bonjour(dns-sd)를 쓴다.
discover_dnssd() {
  local browse resolve pid instance hostport
  browse="$(mktemp)"; resolve="$(mktemp)"
  trap "rm -f '$browse' '$resolve'" RETURN

  dns-sd -B "$MDNS_SERVICE" local >"$browse" 2>&1 &
  pid=$!
  instance="$(wait_for "$browse" "awk '\$2 == \"Add\" { print \$NF; exit }'")" || true
  kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
  [ -n "$instance" ] || return 1

  dns-sd -L "$instance" "$MDNS_SERVICE" local >"$resolve" 2>&1 &
  pid=$!
  hostport="$(wait_for "$resolve" "sed -n 's/.*can be reached at \([^ ][^ ]*\).*/\1/p' | head -1")" || true
  kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
  [ -n "$hostport" ] || return 1

  # "Android.local.:33661" 처럼 끝에 점이 붙어서 온다.
  hostport="${hostport/.:/:}"
  local host="${hostport%:*}" port="${hostport##*:}" ip
  ip="$(dscacheutil -q host -a name "$host" 2>/dev/null | awk '/^ip_address:/ { print $2; exit }')"
  printf '%s\n' "${ip:-$host}:$port"
}

# 그 밖의 OS: adb mdns services 출력에서 주소를 뽑는다.
discover_adb() {
  adb mdns services 2>/dev/null |
    awk -v svc="$MDNS_SERVICE" '$2 ~ svc { print $NF; exit }'
}

endpoint="$(live_endpoint)"
if [ -n "$endpoint" ]; then
  info "이미 연결됨: $endpoint"
  printf '%s\n' "$endpoint"
  exit 0
fi

if [ -r "$CACHE_FILE" ]; then
  cached="$(cat "$CACHE_FILE")"
  if try_connect "$cached"; then
    info "직전 주소로 연결: $cached"
    printf '%s\n' "$cached"
    exit 0
  fi
  adb disconnect "$cached" >/dev/null 2>&1 || true
fi

info "mDNS 로 기기를 찾는다..."
if command -v dns-sd >/dev/null 2>&1; then
  endpoint="$(discover_dnssd || true)"
else
  endpoint="$(discover_adb || true)"
fi

if [ -z "$endpoint" ] || ! try_connect "$endpoint"; then
  warn "기기에 붙지 못했다. 확인할 것:"
  warn "  1) 폰: 설정 > 개발자 옵션 > 무선 디버깅 ON"
  warn "  2) 폰과 PC 가 같은 Wi-Fi (게스트망/AP 격리 주의)"
  warn "  3) 페어링이 풀렸으면: scripts/connect.sh pair <ip:port> <code>"
  exit 1
fi

mkdir -p "$(dirname "$CACHE_FILE")"
printf '%s\n' "$endpoint" >"$CACHE_FILE"
model="$(adb -s "$endpoint" shell getprop ro.product.model 2>/dev/null | tr -d '\r')"
info "연결 완료: ${model:-unknown} ($endpoint)"
printf '%s\n' "$endpoint"
