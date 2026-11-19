default:
    @just --list

# Build native Linux binary
build-linux:
    cargo build --release -p studio-launcher
    @mkdir -p dist
    @cp target/release/studio-launcher dist/rtrust

# Build standalone Windows single-binary RTRUST.exe
build-win: build-windows

build-windows:
    @echo "==> Building embedded stub WebView2Loader.dll for Windows..."
    cargo build --release -p webview2-bridge --target x86_64-pc-windows-gnu
    @mkdir -p assets/dll
    @cp target/x86_64-pc-windows-gnu/release/WebView2Loader.dll assets/dll/WebView2Loader.dll
    @echo "==> Building Rust DXGI Graphics & Resource Shim (dxgi.dll)..."
    cargo build --release -p dxgi-shim --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/dxgi.dll assets/dll/dxgi.dll
    @echo "==> Building standalone RTRUST.exe for Windows..."
    cargo build --release -p studio-launcher --target x86_64-pc-windows-gnu
    @cp target/x86_64-pc-windows-gnu/release/studio-launcher.exe target/x86_64-pc-windows-gnu/release/RTRUST.exe
    @mkdir -p dist
    @cp target/x86_64-pc-windows-gnu/release/RTRUST.exe dist/
    @cp target/x86_64-pc-windows-gnu/release/dxgi.dll dist/
    @echo "==> Standalone Windows executable ready at dist/RTRUST.exe and dist/dxgi.dll"

# Build both Linux launcher and Windows standalone executable
build-all: build-linux build-win

# Check workspace compilation for Linux and Windows
check:
    cargo check -p studio-launcher
    cargo check -p studio-launcher --target x86_64-pc-windows-gnu
    cargo check -p dxgi-shim --target x86_64-pc-windows-gnu
    cargo check -p webview2-bridge --target x86_64-pc-windows-gnu

# Package distribution files into dist/
package: build-linux build-win
    mkdir -p dist
    cp target/release/studio-launcher dist/rtrust
    cp target/x86_64-pc-windows-gnu/release/RTRUST.exe dist/RTRUST.exe
    cp target/x86_64-pc-windows-gnu/release/WebView2Loader.dll dist/WebView2Loader.dll
    cp target/x86_64-pc-windows-gnu/release/dxgi.dll dist/dxgi.dll
    cp assets/rtrust.svg dist/
    @echo "Release bundle ready in dist/ (Linux: rtrust, Windows: RTRUST.exe, dxgi.dll)"

# Clean build artifacts
clean:
    cargo clean
    rm -rf dist
