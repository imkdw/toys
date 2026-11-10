#!/usr/bin/env bash
# 다른 맥에 설치할 DMG 만들기.
#
#   scripts/installer.sh    -> target/dist/TypingPet-<버전>.dmg
#
# DMG 안: TypingPet.app (universal 바이너리, 캐릭터 이미지 전부 Resources 에 포함), 설치.command, 읽어주세요.txt
# 받는 맥에서는 설치.command 를 실행하면 /Applications 에 복사하고 격리 속성을 지운 뒤 실행한다.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

scripts/bundle.sh --universal
APP="target/bundle/TypingPet.app"

# 캐릭터 이미지가 빠짐없이 들어갔는지 확인 (manifest 가 있는 폴더마다 layers 의 png 개수를 비교)
missing=0
for d in characters/*/; do
  name="$(basename "$d")"
  [[ "$name" == _* || ! -f "$d/manifest.toml" ]] && continue
  src=$(find "$d/layers" -name '*.png' | wc -l | tr -d ' ')
  dst=$(find "$APP/Contents/Resources/characters/$name/layers" -name '*.png' 2>/dev/null | wc -l | tr -d ' ')
  echo "캐릭터 $name: 이미지 $dst/$src"
  [[ "$src" == "$dst" && "$src" != 0 ]] || missing=1
done
[[ "$missing" == 0 ]] || { echo "이미지가 빠졌음" >&2; exit 1; }
lipo -info "$APP/Contents/MacOS/typing-pet"

version="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
DIST="target/dist"
STAGE="$DIST/stage"
DMG="$DIST/TypingPet-$version.dmg"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"

cat >"$STAGE/설치.command" <<'EOF'
#!/bin/bash
# TypingPet 설치: /Applications 에 복사하고, 다른 맥에서 받은 앱이라 붙는 격리 속성을 지운 뒤 실행한다.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
DEST="/Applications"
[[ -w "$DEST" ]] || DEST="$HOME/Applications"
mkdir -p "$DEST"
pkill -x typing-pet 2>/dev/null || true
rm -rf "$DEST/TypingPet.app"
cp -R "$HERE/TypingPet.app" "$DEST/"
xattr -dr com.apple.quarantine "$DEST/TypingPet.app" 2>/dev/null || true
echo "설치 완료: $DEST/TypingPet.app"
echo
echo "처음 실행하면 '입력 모니터링' 권한을 물어본다."
echo "시스템 설정 > 개인정보 보호 및 보안 > 입력 모니터링 에서 TypingPet 을 켠 뒤 앱을 다시 실행하면 된다."
open "$DEST/TypingPet.app"
EOF
chmod +x "$STAGE/설치.command"

cat >"$STAGE/읽어주세요.txt" <<'EOF'
TypingPet 설치

1. 설치.command 를 더블클릭한다.
   "확인되지 않은 개발자" 경고가 나오면: 설치.command 를 우클릭 > 열기 > 열기.
   그래도 막히면 터미널에서:  bash "/Volumes/TypingPet/설치.command"
2. 시스템 설정 > 개인정보 보호 및 보안 > 입력 모니터링 에서 TypingPet 을 켠다.
3. TypingPet 을 다시 실행한다 (응용 프로그램 폴더). 메뉴바 아이콘에서 캐릭터를 고를 수 있다.

지우기: 응용 프로그램 폴더에서 TypingPet.app 을 휴지통으로.
EOF

hdiutil create -volname TypingPet -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"
du -h "$DMG"
echo "$DMG"
