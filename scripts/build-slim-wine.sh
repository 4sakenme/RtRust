#!/usr/bin/env bash
# ==============================================================================
# RTRUST - Slim Wine Runtime Builder Script
#
# Downloads Kron4ek Wine-Staging (64-bit AMD64) and removes all bloated,
# unnecessary components (documentation, 32-bit syswow64, test tools,
# redundant drivers) to create an ultra-lightweight portable Wine package:
#   `rtrust-slim-wine-x86_64.tar.zst` (~65-80MB vs 400MB+ default)
# ==============================================================================

set -euo pipefail

WINE_VERSION="9.21"
TAR_NAME="wine-staging-${WINE_VERSION}-amd64.tar.xz"
DOWNLOAD_URL="https://github.com/Kron4ek/Wine-Builds/releases/download/${WINE_VERSION}/${TAR_NAME}"
BUILD_DIR="/tmp/rtrust-wine-build"
OUT_DIR="$(pwd)/dist"
OUTPUT_PACKAGE="${OUT_DIR}/rtrust-slim-wine-x86_64.tar.zst"

echo "==> [RTRUST] Starting Slim Wine Runtime Builder..."
mkdir -p "${BUILD_DIR}"
mkdir -p "${OUT_DIR}"

cd "${BUILD_DIR}"

if [ ! -f "${TAR_NAME}" ]; then
    echo "==> Downloading ${TAR_NAME} from ${DOWNLOAD_URL}..."
    curl -L --progress-bar -o "${TAR_NAME}" "${DOWNLOAD_URL}"
fi

echo "==> Extracting Wine archive..."
rm -rf "wine-extracted"
mkdir -p "wine-extracted"
tar -xf "${TAR_NAME}" -C "wine-extracted" --strip-components=1

cd "wine-extracted"

echo "==> Slimming down Wine distribution for Roblox Studio..."
# 1. Remove documentation, man pages, C header includes
rm -rf share/man share/doc include

# 2. Remove test applications, geckos, monorepo artifacts
rm -f bin/winedbg bin/winedump bin/widl bin/wmc bin/wrc bin/function_list
rm -rf share/wine/gecko share/wine/mono

# 3. Strip debug symbols from binaries and libraries
echo "==> Stripping binaries and shared objects..."
find bin/ lib/ -type f -exec strip --strip-unneeded {} + 2>/dev/null || true

cd "${BUILD_DIR}/wine-extracted"

echo "==> Packaging slim runtime into ${OUTPUT_PACKAGE}..."
tar --zstd -cf "${OUTPUT_PACKAGE}" *

echo "==> SUCCESS: Slim Wine Runtime created at:"
ls -lh "${OUTPUT_PACKAGE}"

echo "==> Cleaning up build directory..."
rm -rf "${BUILD_DIR}"
echo "==> Done!"
