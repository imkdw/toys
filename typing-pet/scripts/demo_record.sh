#!/usr/bin/env bash
# 동작 영상 녹화: 시연용 인스턴스를 띄우고 probe demo 시나리오를 보내며 화면 일부를 녹화한다.
# 결과: target/demo/typing-pet-demo.mp4 (+ .gif)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
V=target/demo
mkdir -p "$V"
cargo build -q --bins
printf 'character = "usagi"\nposition = [580.0, 150.0]\nsize = 320.0\nvisible = true\nbreathing = true\n' >"$V/config.toml"
TYPING_PET_CONFIG="$V/config.toml" ./target/debug/typing-pet >"$V/app.log" 2>&1 &
APP=$!
sleep 3
ffmpeg -hide_banner -loglevel error -y -f avfoundation -pixel_format uyvy422 -framerate 60 -capture_cursor 1 -i "2:none" \
  -vf "crop=840:440:100:100,format=yuv420p" -c:v libx264 -preset veryfast -crf 18 "$V/raw.mp4" >"$V/ffmpeg.log" 2>&1 &
FF=$!
sleep 1.5
./target/debug/probe demo --trace "$V/none.jsonl" >"$V/probe.log" 2>&1 &
PR=$!
# probe 창을 맨 앞으로 (macOS 가 백그라운드 앱의 포커스 가져가기를 막는 경우가 있어서)
for _ in $(seq 1 20); do
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $PR) to true" >/dev/null 2>&1
  sleep 0.5
  grep -q . "$V/probe.log" && break
  kill -0 $PR 2>/dev/null || break
  # 시작했으면 시나리오가 끝날 때까지 기다림
  [[ -n "$(pgrep -P $PR)" ]] && break
done
for _ in $(seq 1 120); do kill -0 $PR 2>/dev/null || break; sleep 1; done
kill $PR 2>/dev/null
kill -INT $FF 2>/dev/null
wait $FF 2>/dev/null
kill $APP 2>/dev/null
ffmpeg -hide_banner -loglevel error -y -i "$V/raw.mp4" -c copy -movflags +faststart "$V/typing-pet-demo.mp4"
ffmpeg -hide_banner -loglevel error -y -i "$V/typing-pet-demo.mp4" -vf "fps=20,scale=560:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse" "$V/typing-pet-demo.gif"
cat "$V/probe.log"
ffprobe -v error -show_entries format=duration -of csv=p=0 "$V/typing-pet-demo.mp4"
