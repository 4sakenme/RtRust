#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="/tmp/ios_crash.log"
PATTERN='RtRustMobile|rustudio|panic|wgpu|Metal|RUST_BACKTRACE|EXC_BAD_ACCESS|SIGABRT|SIGSEGV'
export PATH="$HOME/.local/bin:$PATH"

if ! command -v pymobiledevice3 >/dev/null 2>&1; then
  echo "Error: 'pymobiledevice3' not found in PATH." >&2
  exit 1
fi

# Ensure usbmuxd is active if systemd is available
if command -v systemctl >/dev/null 2>&1 && ! systemctl is-active --quiet usbmuxd; then
  echo "[Wi-Fi Log] Starting usbmuxd..."
  sudo systemctl start usbmuxd || true
fi

# Start pymobiledevice3 remote tunneld if not already running
if ! pgrep -f "pymobiledevice3 remote tunneld" >/dev/null 2>&1; then
  echo "[Wi-Fi Log] Starting pymobiledevice3 remote tunneld (requires sudo)..."
  sudo -b pymobiledevice3 remote tunneld >/tmp/pmd3_tunneld.log 2>&1
  sleep 2
fi

echo "[Wi-Fi Log] Streaming live iOS syslog over Wi-Fi to ${LOG_FILE} (Ctrl+C to stop)..."
: > "$LOG_FILE"
stdbuf -oL pymobiledevice3 syslog live 2>&1 \
  | grep --line-buffered -iE "$PATTERN" \
  | tee -a "$LOG_FILE"
