#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p android/jniLibs dist

echo "==> Checking Android build prerequisites..."
for cmd in rustup cargo; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Error: '$cmd' is not installed."
    exit 1
  fi
done

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "Installing cargo-ndk..."
  cargo install cargo-ndk
fi

if [[ -z "${ANDROID_NDK_HOME:-}" && -z "${NDK_HOME:-}" ]]; then
  echo "Warning: ANDROID_NDK_HOME is not set. Make sure Android NDK r26+ is installed and exported."
fi

echo "==> Adding Rust Android targets (x86_64 for Waydroid, aarch64 for ARM64 phones)..."
rustup target add x86_64-linux-android aarch64-linux-android

echo "==> Compiling librtrust_mobile.so for x86_64 & arm64-v8a (--profile mobile-release)..."
cargo ndk \
  -t x86_64 \
  -t arm64-v8a \
  -o android/jniLibs \
  build --profile mobile-release --lib

echo "==> Built shared libraries:"
ls -lh android/jniLibs/x86_64/librtrust_mobile.so android/jniLibs/arm64-v8a/librtrust_mobile.so

echo "==> Packaging APK..."
if [[ -x android/gradlew ]]; then
  (cd android && ./gradlew assembleDebug)
  cp android/build/outputs/apk/debug/android-debug.apk dist/RtRustMobile-debug.apk
elif command -v gradle >/dev/null 2>&1; then
  (cd android && gradle assembleDebug)
  cp android/build/outputs/apk/debug/android-debug.apk dist/RtRustMobile-debug.apk
else
  echo "Note: Gradle not found in PATH. Shared libraries are ready in android/jniLibs/."
  echo "Install Gradle or Android Studio to generate dist/RtRustMobile-debug.apk."
  exit 0
fi

echo "==> APK ready: dist/RtRustMobile-debug.apk"

if command -v adb >/dev/null 2>&1 && adb get-state >/dev/null 2>&1; then
  echo "==> Installing dist/RtRustMobile-debug.apk onto connected Android/Waydroid device..."
  adb install -r dist/RtRustMobile-debug.apk
else
  echo "Note: No active adb/Waydroid device detected. Connect Waydroid and run:"
  echo "  adb install -r dist/RtRustMobile-debug.apk"
fi
