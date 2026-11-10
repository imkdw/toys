#!/usr/bin/env bash
# .app 번들 만들기 (PLAN 7단계). cargo-bundle 없이 직접 구성한다.
#
#   scripts/bundle.sh               릴리스 빌드 -> target/bundle/TypingPet.app, 고정 인증서로 서명
#   scripts/bundle.sh --make-cert   서명용 자체 서명 인증서를 로그인 키체인에 만든다 (처음 한 번)
#   scripts/bundle.sh --install     만든 번들을 ~/Applications 에 복사
#   scripts/bundle.sh --debug       디버그 빌드로 번들 (트레이스 포함, verify.sh --live --bundle 용). target/bundle-debug/
#   scripts/bundle.sh --universal   Apple Silicon + Intel 둘 다 도는 바이너리로 번들 (다른 맥 배포용, scripts/installer.sh 가 쓴다)
#
# 같은 인증서로 계속 서명해야 다시 빌드해도 macOS 가 "입력 모니터링" 권한을 유지한다.
# (임시 서명(ad-hoc)은 빌드마다 코드 해시가 바뀌어 권한을 다시 요구한다.)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
IDENTITY="typing-pet dev"
APP="target/bundle/TypingPet.app"
make_cert=0
install=0
debug=0
universal=0
for a in "$@"; do
  case "$a" in
    --make-cert) make_cert=1 ;;
    --install) install=1 ;;
    --debug) debug=1; APP="target/bundle-debug/TypingPet.app" ;;
    --universal) universal=1 ;;
    *) echo "알 수 없는 인자: $a" >&2; exit 2 ;;
  esac
done

if [[ "$make_cert" == 1 ]]; then
  if security find-certificate -c "$IDENTITY" >/dev/null 2>&1; then
    echo "이미 있음: $IDENTITY"
  else
    tmp="$(mktemp -d)"
    cat >"$tmp/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $IDENTITY
[ext]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,codeSigning
EOF
    openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -config "$tmp/cert.cnf" -keyout "$tmp/key.pem" -out "$tmp/cert.pem" 2>/dev/null
    openssl pkcs12 -export -legacy -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -name "$IDENTITY" -out "$tmp/id.p12" -passout pass:typingpet 2>/dev/null \
      || openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" -name "$IDENTITY" -out "$tmp/id.p12" -passout pass:typingpet
    security import "$tmp/id.p12" -k "$HOME/Library/Keychains/login.keychain-db" -P typingpet -T /usr/bin/codesign
    rm -rf "$tmp"
    echo "인증서 생성: $IDENTITY"
  fi
fi

if [[ "$universal" == 1 ]]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target list --installed | grep -qx "$t" || rustup target add "$t"
    cargo build --release --bin typing-pet --target "$t"
  done
  mkdir -p target/universal
  bin="target/universal/typing-pet"
  lipo -create -output "$bin" target/aarch64-apple-darwin/release/typing-pet target/x86_64-apple-darwin/release/typing-pet
elif [[ "$debug" == 1 ]]; then
  cargo build --bin typing-pet
  bin="target/debug/typing-pet"
else
  cargo build --release --bin typing-pet
  bin="target/release/typing-pet"
fi
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/characters"
cp "$bin" "$APP/Contents/MacOS/typing-pet"
# 앱 아이콘 (우사기 레이어로 만든다. 비공개 에셋이라 저장소에 두지 않고 target/icon 에 매번 만든다)
python3 scripts/make_icon.py >/dev/null
cp target/icon/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
# 캐릭터: 테스트용(_*)과 기준 이미지는 넣지 않는다
for d in characters/*/; do
  name="$(basename "$d")"
  [[ "$name" == _* || ! -f "$d/manifest.toml" ]] && continue
  mkdir -p "$APP/Contents/Resources/characters/$name"
  cp "$d/manifest.toml" "$APP/Contents/Resources/characters/$name/"
  cp -R "$d/layers" "$APP/Contents/Resources/characters/$name/"
done
version="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
cat >"$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>TypingPet</string>
  <key>CFBundleDisplayName</key><string>TypingPet</string>
  <key>CFBundleIdentifier</key><string>dev.imkdw.typing-pet</string>
  <key>CFBundleExecutable</key><string>typing-pet</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

if security find-certificate -c "$IDENTITY" >/dev/null 2>&1; then
  codesign --force --deep --options runtime --timestamp=none -s "$IDENTITY" "$APP"
  echo "서명: $IDENTITY"
else
  codesign --force --deep -s - "$APP"
  echo "경고: 고정 인증서가 없어 임시 서명함. 다시 빌드하면 권한을 다시 요구한다 (scripts/bundle.sh --make-cert)"
fi
codesign -dv "$APP" 2>&1 | grep -E "Identifier|Authority|Signature" || true

if [[ "$install" == 1 ]]; then
  mkdir -p "$HOME/Applications"
  rm -rf "$HOME/Applications/TypingPet.app"
  cp -R "$APP" "$HOME/Applications/"
  echo "설치: ~/Applications/TypingPet.app"
fi
echo "$APP"
