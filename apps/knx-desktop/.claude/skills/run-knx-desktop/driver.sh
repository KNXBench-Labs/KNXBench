#!/usr/bin/env bash
# Driver for the KNXBench Tauri desktop app (Hyprland/Wayland).
# Subcommands: start | stop | status | shot [file] | key <combo> | type <text>
#
# Requires: cargo-tauri CLI, npm deps installed, Hyprland (hyprctl), grim, wtype.
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"   # apps/knx-desktop
LOG_FILE="/tmp/knx-desktop-driver.log"
PID_FILE="/tmp/knx-desktop-driver.pid"
WINDOW_TITLE="KNXBench"

find_window() {
  hyprctl clients -j | python3 -c "
import json, sys
d = json.load(sys.stdin)
for c in d:
    if c['title'] == '$WINDOW_TITLE':
        x, y = c['at']
        w, h = c['size']
        print(f'{x} {y} {w} {h}')
        break
"
}

cmd_start() {
  if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
    echo "already running (pid $(cat "$PID_FILE"))"
    return 0
  fi
  cd "$APP_DIR"
  # WEBKIT_DISABLE_DMABUF_RENDERER=1 works around a blank/black webview
  # caused by GBM buffer allocation failures on this GPU/compositor combo.
  (WEBKIT_DISABLE_DMABUF_RENDERER=1 nohup cargo tauri dev >"$LOG_FILE" 2>&1 & echo $! >"$PID_FILE")
  echo "launching (cargo-tauri pid $(cat "$PID_FILE")), waiting for window..."
  for _ in $(seq 1 60); do
    sleep 2
    if find_window >/dev/null 2>&1 && [ -n "$(find_window)" ]; then
      echo "window up: $(find_window)"
      return 0
    fi
    if grep -qE "^error(\[|:)" "$LOG_FILE" 2>/dev/null; then
      echo "build/run error, see $LOG_FILE"
      tail -30 "$LOG_FILE"
      return 1
    fi
  done
  echo "timed out waiting for window, see $LOG_FILE"
  return 1
}

cmd_stop() {
  pkill -f "cargo-tauri tauri dev" 2>/dev/null || true
  pkill -f "target/debug/knx-desktop" 2>/dev/null || true
  pkill -f "knx-desktop/node_modules/.bin/vite" 2>/dev/null || true
  rm -f "$PID_FILE"
  echo "stopped"
}

cmd_status() {
  local geom
  geom="$(find_window || true)"
  if [ -n "$geom" ]; then
    echo "window: $geom (x y w h)"
  else
    echo "window: not found"
  fi
  pgrep -af "cargo-tauri tauri dev|target/debug/knx-desktop" || echo "no processes"
}

cmd_shot() {
  local out="${1:-/tmp/knx-desktop-shot.png}"
  local geom
  geom="$(find_window)"
  if [ -z "$geom" ]; then
    echo "window not found, is it running? (driver.sh start)" >&2
    return 1
  fi
  read -r x y w h <<<"$geom"
  grim -g "${x},${y} ${w}x${h}" "$out"
  echo "$out"
}

cmd_key() {
  # combo like "ctrl+k" or "ctrl+shift+p"
  local combo="$1"
  local mods=()
  local key=""
  IFS='+' read -ra parts <<<"$combo"
  for p in "${parts[@]}"; do
    case "$p" in
      ctrl|shift|alt|super) mods+=("$p") ;;
      *) key="$p" ;;
    esac
  done
  local wtype_args=()
  for m in "${mods[@]}"; do wtype_args+=(-M "$m"); done
  wtype_args+=(-k "$key")
  for m in "${mods[@]}"; do wtype_args+=(-m "$m"); done
  wtype "${wtype_args[@]}"
}

cmd_type() {
  wtype "$1"
}

case "${1:-}" in
  start) cmd_start ;;
  stop) cmd_stop ;;
  status) cmd_status ;;
  shot) shift; cmd_shot "$@" ;;
  key) shift; cmd_key "$@" ;;
  type) shift; cmd_type "$@" ;;
  *)
    echo "usage: $0 {start|stop|status|shot [file]|key <ctrl+k>|type <text>}" >&2
    exit 1
    ;;
esac
