export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/bin:$(PATH)

.PHONY: all build-linux build-win build-windows package clean check help

all: build-linux build-win

build-linux:
	cargo build --release -p studio-launcher
	@mkdir -p dist
	@cp target/release/studio-launcher dist/rtrust

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

check:
	cargo check -p studio-launcher
	cargo check -p studio-launcher --target x86_64-pc-windows-gnu
	cargo check -p dxgi-shim --target x86_64-pc-windows-gnu
	cargo check -p webview2-bridge --target x86_64-pc-windows-gnu

package: build-linux build-win
	@mkdir -p dist
	@cp target/release/studio-launcher dist/rtrust
	@cp target/x86_64-pc-windows-gnu/release/RTRUST.exe dist/RTRUST.exe
	@cp target/x86_64-pc-windows-gnu/release/WebView2Loader.dll dist/WebView2Loader.dll
	@cp target/x86_64-pc-windows-gnu/release/dxgi.dll dist/dxgi.dll
	@cp assets/rtrust.svg dist/
	@echo "Packaged artifacts ready in dist/:"
	@ls -lh dist/

clean:
	cargo clean
	@rm -rf dist

help:
	@echo "Available targets:"
	@echo "  build-linux    Build native Linux studio launcher binary (dist/rtrust)"
	@echo "  build-win      Cross-compile standalone Windows RTRUST.exe with embedded WebUI & stub DLL"
	@echo "  package        Bundle Linux & Windows release binaries into dist/"
	@echo "  check          Run cargo check for both Linux and Windows targets"
	@echo "  clean          Remove compilation artifacts"
