#!/usr/bin/env bash
# 다른 맥에 설치할 DMG 만들기.
#
#   scripts/installer.sh    -> target/dist/TypingPet-<버전>.dmg
#
# DMG 안: TypingPet.app (universal 바이너리, 캐릭터 이미지 전부 Resources 에 포함), 응용 프로그램 바로가기 둘만 둔다.
# 창을 열면 캐릭터 배경 위에서 앱을 응용 프로그램으로 끌어 놓으면 된다.
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
if [[ -f characters/_props/props.toml ]]; then
  src=$(find characters/_props/layers -name '*.png' | wc -l | tr -d ' ')
  dst=$(find "$APP/Contents/Resources/characters/_props/layers" -name '*.png' 2>/dev/null | wc -l | tr -d ' ')
  echo "책상 소품: 이미지 $dst/$src"
  [[ "$src" == "$dst" ]] || missing=1
fi
[[ "$missing" == 0 ]] || { echo "이미지가 빠졌음" >&2; exit 1; }
lipo -info "$APP/Contents/MacOS/typing-pet"

version="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
DIST="target/dist"
STAGE="$DIST/stage"
DMG="$DIST/TypingPet-$version.dmg"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"


# 설치 창: 캐릭터 배경 위에 앱 -> 응용 프로그램 으로 끌어 놓게 꾸민다 (art/_app/final/dmg_bg.png, 없으면 기본 창)
PY="art/.venv/bin/python"
BG_SRC="art/_app/final/dmg_bg.png"
if [[ -f "$BG_SRC" ]] && "$PY" -c "import dmgbuild" 2>/dev/null; then
  "$PY" - "$BG_SRC" "$DIST/bg.png" "$DIST/bg@2x.png" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB").resize((1536, 1024), Image.LANCZOS)
im.save(sys.argv[3])
im.resize((768, 512), Image.LANCZOS).save(sys.argv[2])
PY
  tiffutil -cathidpicheck "$DIST/bg.png" "$DIST/bg@2x.png" -out "$DIST/bg.tiff" >/dev/null
  "$PY" -m dmgbuild -s scripts/dmg_settings.py \
    -D app="$STAGE/TypingPet.app" \
    -D bg="$DIST/bg.tiff" -D icon="$STAGE/TypingPet.app/Contents/Resources/AppIcon.icns" \
    TypingPet "$DMG" >/dev/null
  rm -f "$DIST/bg.png" "$DIST/bg@2x.png" "$DIST/bg.tiff"
else
  echo "경고: $BG_SRC 나 dmgbuild 가 없어 꾸미지 않은 DMG 를 만든다 (uv pip install --python $PY dmgbuild)" >&2
  ln -s /Applications "$STAGE/Applications"
  hdiutil create -volname TypingPet -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
fi
rm -rf "$STAGE"
du -h "$DMG"
echo "$DMG"
