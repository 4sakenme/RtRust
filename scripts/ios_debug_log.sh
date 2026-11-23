#!/usr/bin/env bash
set -euo pipefail

LOG_FILE="/tmp/ios_crash.log"

if ! command -v idevicesyslog >/dev/null 2>&1; then
    echo "idevicesyslog not found. Install libimobiledevice on Arch Linux:"
    echo "  sudo pacman -S libimobiledevice usbmuxd"
    exit 1
fi

echo "Capturing iPhone USB logs (RtRustMobile / panic / wgpu / Metal / RUST_BACKTRACE)..."
echo "Writing to: $LOG_FILE"
echo "Press Ctrl+C to stop."

: > "$LOG_FILE"
idevicesyslog | grep --line-buffered -iE "RtRustMobile|rustudio|panic|wgpu|Metal|RUST_BACKTRACE|EXC_BAD_ACCESS|SIGABRT|SIGSEGV" | tee -a "$LOG_FILE"
