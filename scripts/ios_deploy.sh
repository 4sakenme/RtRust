#!/usr/bin/env bash
set -euo pipefail

REPO="4sakenme/RtRust"
WORKFLOW="build-ios.yml"
BRANCH="mobile"
ARTIFACT="RtRustMobile-unsigned-ipa"
DIST_DIR="./dist/ios"
IPA_PATH="${DIST_DIR}/RtRustMobile-unsigned.ipa"
LOG_FILE="/tmp/ios_crash.log"

DO_BUILD=0
DO_LOG=0
SIDELOADER_ARGS=()

for arg in "$@"; do
  case "$arg" in
    --build) DO_BUILD=1 ;;
    --log) DO_LOG=1 ;;
    -h|--help)
      echo "Usage: $0 [--build] [--log] [--udid <UDID>]"
      echo "  --build   Trigger a new GitHub Actions iOS build and wait for it"
      echo "  --log     Stream filtered iPhone syslog to ${LOG_FILE} after install"
      exit 0
      ;;
    *) SIDELOADER_ARGS+=("$arg") ;;
  esac
done

export PATH="$HOME/.local/bin:$PATH"

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: 'gh' CLI not found in PATH." >&2
  exit 1
fi
if ! command -v sideloader >/dev/null 2>&1; then
  echo "Error: 'sideloader' CLI not found in PATH (expected in ~/.local/bin/sideloader)." >&2
  exit 1
fi

# Ensure usbmuxd is active if systemd is available
if command -v systemctl >/dev/null 2>&1 && ! systemctl is-active --quiet usbmuxd; then
  echo "[iOS Deploy] Starting usbmuxd..."
  sudo systemctl start usbmuxd || true
fi

# Step A: Fetch Latest Build
if [[ "$DO_BUILD" -eq 1 ]]; then
  echo "[Step A] Triggering ${WORKFLOW} on ${REPO} (${BRANCH})..."
  gh workflow run "$WORKFLOW" -R "$REPO" --ref "$BRANCH"
  sleep 4
  RUN_ID="$(gh run list -R "$REPO" --workflow="$WORKFLOW" --branch="$BRANCH" --limit=1 --json databaseId --jq '.[0].databaseId')"
  echo "[Step A] Watching run ${RUN_ID}..."
  gh run watch "$RUN_ID" -R "$REPO" --exit-status
else
  RUN_ID="$(gh run list -R "$REPO" --workflow="$WORKFLOW" --branch="$BRANCH" --status=success --limit=1 --json databaseId --jq '.[0].databaseId')"
fi

if [[ -z "${RUN_ID:-}" || "$RUN_ID" == "null" ]]; then
  echo "Error: No successful '${WORKFLOW}' run found on branch '${BRANCH}'. Run with --build first." >&2
  exit 1
fi

echo "[Step A] Downloading ${ARTIFACT} from run ${RUN_ID} into ${DIST_DIR}..."
mkdir -p "$DIST_DIR"
rm -f "${DIST_DIR}"/*.ipa
gh run download "$RUN_ID" -R "$REPO" -n "$ARTIFACT" -D "$DIST_DIR"

if [[ ! -f "$IPA_PATH" ]]; then
  echo "Error: Expected IPA not found at ${IPA_PATH}" >&2
  exit 1
fi
ls -lh "$IPA_PATH"

# Step B: Sign & Push to iPhone
echo "[Step B] Signing and installing ${IPA_PATH} onto iPhone..."
sideloader install -i "${SIDELOADER_ARGS[@]}" "$IPA_PATH"

# Step C: Optional Live Log
if [[ "$DO_LOG" -eq 1 ]]; then
  echo "[Step C] Streaming filtered iPhone logs to ${LOG_FILE} (Ctrl+C to stop)..."
  : > "$LOG_FILE"
  if command -v pymobiledevice3 >/dev/null 2>&1; then
    stdbuf -oL pymobiledevice3 syslog live 2>&1 \
      | grep --line-buffered -iE 'RtRustMobile|rustudio|panic|wgpu|Metal|RUST_BACKTRACE|EXC_BAD_ACCESS|SIGABRT|SIGSEGV' \
      | tee -a "$LOG_FILE"
  elif command -v idevicesyslog >/dev/null 2>&1; then
    stdbuf -oL idevicesyslog 2>&1 \
      | grep --line-buffered -iE 'RtRustMobile|rustudio|panic|wgpu|Metal|RUST_BACKTRACE|EXC_BAD_ACCESS|SIGABRT|SIGSEGV' \
      | tee -a "$LOG_FILE"
  else
    echo "Error: Neither 'pymobiledevice3' nor 'idevicesyslog' found for live logging." >&2
    exit 1
  fi
fi
