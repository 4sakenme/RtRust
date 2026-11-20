#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/4sakenme/RtRust.git"
SRC_DIR="${RTRUST_SRC_DIR:-$HOME/.local/share/rtrust-src}"

CYAN='\033[1;36m'
BLUE='\033[1;34m'
ORANGE='\033[38;5;208m'
GREEN='\033[1;32m'
RED='\033[1;31m'
WHITE='\033[1;37m'
GRAY='\033[0;90m'
RESET='\033[0m'

echo ""
echo -e "        ${GRAY}╭──────────────────────────╮${RESET}"
echo -e "        ${GRAY}│${RESET}      ${CYAN}▄▄█████████▄▄${RESET}       ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}    ${CYAN}▄███▀▀     ▀▀███▄${RESET}     ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}   ${CYAN}███${RESET}     ${WHITE}████${RESET}     ${BLUE}███${ORANGE}▀▄${RESET}  ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}   ${CYAN}███${RESET}     ${WHITE}████${RESET}  ${GREEN}│${RESET}  ${BLUE}███${ORANGE}▀█${RESET}  ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}    ${BLUE}▀███▄▄   ${BLUE}▄${WHITE}●${RED}╲${BLUE}▄███▀${ORANGE}▀█${RESET}  ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}      ${BLUE}▀▀█████████▀▀${ORANGE} ▄█▀${RESET}   ${GRAY}│${RESET}"
echo -e "        ${GRAY}│${RESET}        ${ORANGE}▀▄▄▄▀▀▄▄▄▀▀${RESET}       ${GRAY}│${RESET}"
echo -e "        ${GRAY}╰──────────────────────────╯${RESET}"
echo -e "${ORANGE}"
cat <<'EOF'
  ____  _   ____            _
 |  _ \| |_|  _ \ _   _ ___| |_
 | |_) | __| |_) | | | / __| __|
 |  _ <| |_|  _ <| |_| \__ \ |_
 |_| \_\\__|_| \_\\__,_|___/\__|
EOF
echo -e "  ${CYAN}Roblox Studio Launcher & Runner for Linux${RESET}"
echo "=================================================="

# Ask sudo password upfront for installing missing system packages & /usr/local/bin
if command -v sudo >/dev/null 2>&1; then
  echo "==> Requesting sudo privileges to check/install missing dependencies and create the Desktop Entry:"
  sudo -v
fi

# 1. Check & install missing system packages per distro
install_deps_arch() {
  local pkgs=(git curl base-devel pkgconf gtk3 webkit2gtk-4.1 openssl wine)
  local missing=()
  for p in "${pkgs[@]}"; do
    pacman -Qi "$p" >/dev/null 2>&1 || missing+=("$p")
  done
  if ((${#missing[@]} > 0)); then
    echo "==> Installing missing Arch packages: ${missing[*]}"
    sudo pacman -S --needed --noconfirm "${missing[@]}"
  else
    echo "==> All required Arch system packages are already installed."
  fi
}

install_deps_debian() {
  local pkgs=(git curl build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libssl-dev wine)
  echo "==> Checking/installing required Debian/Ubuntu packages..."
  sudo apt-get update -qq
  sudo apt-get install -y "${pkgs[@]}"
}

install_deps_fedora() {
  local pkgs=(git curl gcc gcc-c++ pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel openssl-devel wine)
  echo "==> Checking/installing required Fedora packages..."
  sudo dnf install -y "${pkgs[@]}"
}

if command -v pacman >/dev/null 2>&1; then
  install_deps_arch
elif command -v apt-get >/dev/null 2>&1; then
  install_deps_debian
elif command -v dnf >/dev/null 2>&1; then
  install_deps_fedora
else
  echo "Warning: Unsupported package manager. Please ensure git, curl, gcc, gtk3, webkit2gtk, openssl, and wine are installed."
fi

# 2. Check & install Rust / Cargo if missing
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
  echo "==> Rust/Cargo not found. Installing rustup automatically..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
else
  echo "==> Rust detected: $(rustc --version)"
fi

# 3. Clone or update RtRust source
if [[ -f "./Cargo.toml" && -d "./crates/studio-launcher" ]]; then
  echo "==> Using local RtRust source in: $(pwd)"
  WORK_DIR="$(pwd)"
else
  if [[ -d "$SRC_DIR/.git" ]]; then
    echo "==> Updating RtRust source in $SRC_DIR..."
    git -C "$SRC_DIR" pull --ff-only || true
  else
    echo "==> Cloning RtRust from $REPO_URL..."
    rm -rf "$SRC_DIR"
    git clone --depth 1 "$REPO_URL" "$SRC_DIR"
  fi
  WORK_DIR="$SRC_DIR"
fi

# 4. Compile RtRust (studio-launcher)
cd "$WORK_DIR"
echo "==> Compiling RtRust (release mode)..."
cargo build --release -p studio-launcher

BIN_PATH="$WORK_DIR/target/release/studio-launcher"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "Error: Compiled binary not found at $BIN_PATH"
  exit 1
fi

# 5. Install binary, icon & Desktop Entry
echo "==> Installing binary, icon, and Desktop Entry..."
sudo install -Dm755 "$BIN_PATH" /usr/local/bin/rtrust

if [[ -f "$WORK_DIR/assets/rtrust.svg" ]]; then
  sudo install -Dm644 "$WORK_DIR/assets/rtrust.svg" /usr/share/icons/hicolor/scalable/apps/rtrust.svg
fi

sudo tee /usr/share/applications/rtrust.desktop >/dev/null <<'EOF'
[Desktop Entry]
Name=RtRust
Comment=Lightweight Roblox Studio Launcher & Runner for Linux
Exec=/usr/local/bin/rtrust %u
Icon=rtrust
Terminal=false
Type=Application
Categories=Game;Development;
MimeType=x-scheme-handler/roblox-studio;x-scheme-handler/roblox-player;
StartupNotify=true
EOF

sudo update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
sudo gtk-update-icon-cache -f /usr/share/icons/hicolor >/dev/null 2>&1 || true

echo ""
echo -e "${GREEN}=================================================="
echo "  RtRust installed successfully!"
echo "  - Launch from App Menu : search 'RtRust'"
echo "  - Or run from terminal : rtrust"
echo -e "==================================================${RESET}"
