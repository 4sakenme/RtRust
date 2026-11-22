#!/usr/bin/env bash
set -euo pipefail

echo "=== [1/6] Installing system tools (tmux, python3-pip, unzip, curl) ==="
sudo apt-get update -y
sudo apt-get install -y tmux python3-pip unzip curl zip libclang-dev pkg-config

echo "=== [2/6] Setting up Rust Android targets & cargo-ndk ==="
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-ndk --locked || cargo install cargo-ndk

echo "=== [3/6] Installing Android SDK Command-line Tools, Platforms 34/35 & NDK ==="
ANDROID_HOME="$HOME/android-sdk"
mkdir -p "$ANDROID_HOME/cmdline-tools"
if [ ! -d "$ANDROID_HOME/cmdline-tools/latest" ]; then
    curl -sL "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" -o /tmp/cmdline-tools.zip
    unzip -q -o /tmp/cmdline-tools.zip -d /tmp/cmdline-unpack
    mv /tmp/cmdline-unpack/cmdline-tools "$ANDROID_HOME/cmdline-tools/latest"
    rm -rf /tmp/cmdline-tools.zip /tmp/cmdline-unpack
fi

export ANDROID_HOME
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"

yes | sdkmanager --licenses >/dev/null 2>&1 || true
sdkmanager --install \
    "platform-tools" \
    "platforms;android-34" \
    "platforms;android-35" \
    "build-tools;34.0.0" \
    "build-tools;35.0.0" \
    "ndk;27.2.12479018"

echo "=== [4/6] Installing Gradle 8.10.2 ==="
GRADLE_HOME="$HOME/gradle-8.10.2"
if [ ! -x "$GRADLE_HOME/bin/gradle" ]; then
    curl -sL "https://services.gradle.org/distributions/gradle-8.10.2-bin.zip" -o /tmp/gradle.zip
    unzip -q -o /tmp/gradle.zip -d "$HOME"
    rm -f /tmp/gradle.zip
fi

echo "=== [5/6] Installing Gemini CLI & python-telegram-bot ==="
npm install -g @google/gemini-cli
pip3 install --break-system-packages --upgrade "python-telegram-bot>=20.0" || pip3 install --upgrade "python-telegram-bot>=20.0"

echo "=== [6/6] Installing Google Cloud CLI (gcloud) ==="
if [ ! -d "$HOME/google-cloud-sdk" ]; then
    curl -sL "https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-cli-linux-x86_64.tar.gz" -o /tmp/gcloud.tar.gz
    tar -xzf /tmp/gcloud.tar.gz -C "$HOME"
    rm -f /tmp/gcloud.tar.gz
    "$HOME/google-cloud-sdk/install.sh" --quiet --path-update=false
fi

ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
if ! grep -q "RTRUST_MOBILE_ENV" "$HOME/.bashrc"; then
    cat >> "$HOME/.bashrc" <<EOF

# >>> RTRUST_MOBILE_ENV >>>
export ANDROID_HOME="$ANDROID_HOME"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="$ANDROID_NDK_HOME"
export PATH="\$HOME/.cargo/bin:$GRADLE_HOME/bin:$HOME/google-cloud-sdk/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/build-tools/35.0.0:\$PATH"
# <<< RTRUST_MOBILE_ENV <<<
EOF
fi

echo "=== Setup complete! ==="
