<p align="center">
  <img src="assets/icons/rtrust.svg" width="128" alt="RtRustMobile icon">
</p>

<h1 align="center">RtRustMobile</h1>

<p align="center">
  <strong>Native Android, Waydroid &amp; Ultra-Low-End Roblox Studio Editor &amp; Runtime in Rust</strong>
</p>

<p align="center">
  <a href="https://github.com/4sakenme/RtRust/releases/tag/v0.3.4-mobile-demo.1"><img src="https://img.shields.io/badge/Pre--release-v0.3.4--mobile--demo.1-2563EB?style=flat-square" alt="Pre-release"></a>
  <a href="https://github.com/rippelz/rustudio"><img src="https://img.shields.io/badge/Engine-rustudio%20v0.3.4-orange?style=flat-square" alt="Engine"></a>
  <img src="https://img.shields.io/badge/Platform-Android%20%7C%20Waydroid%20%7C%20Linux-3DDC84?style=flat-square" alt="Platforms">
  <img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-lightgrey?style=flat-square" alt="License">
</p>

---

## What is RtRustMobile?

**RtRustMobile** is the mobile (Android / Waydroid) and ultra-low-end edition of **[RtRust](https://github.com/4sakenme/RtRust)**, powered by the open-source **[rustudio (`v0.3.4`)](https://github.com/rippelz/rustudio)** native Rust engine.

- **Upstream Engine (`rippelz/rustudio v0.3.4`):** Provides the core Roblox Studio runtime written completely in Rust (no Wine) — `.rbxl`/`.rbxlx` & `.rbxm`/`.rbxmx` editing, real Luau VM (`--!native` codegen), Rapier3D & Studio physics, R6/R15 character simulation, `wgpu` rendering, incremental voxel lighting (~97% less CPU), multi-threaded World GUI layout, Team Create live sync, Rojo/Argon File Sync, MicroProfiler, and built-in MCP server.
- **RtRustMobile Layer (`mobile` branch):** Adapts the engine for touchscreens, Android NDK (`arm64-v8a` & `x86_64`), Waydroid, and low-spec laptops with aggressive thread/shader tuning, immersive fullscreen, touch camera & play controls, and Android-safe storage/networking.

---

## Performance Benchmarks (Play Mode — `v0.3.4`)

Measured with `--demo --play --headless --no-ui --bench --seconds 3` across 4 hardware tiers:

| Profile Tier | Environment Config | Frames (3s) | Avg FPS | 1% Low FPS | Avg Frame | Work / CPU | Draw Calls |
|---|---|---:|---:|---:|---:|---:|---:|
| **1. Ultra-Low** | `LOWEND=1`, `PLAY_WORKER=0`, `2T` | **459** | **152.9 FPS** | **96.2 FPS** | **6.54 ms** | **1.40 / 2.26 ms** | **20** |
| **2. Low-End** | `LOWEND=1`, `PLAY_WORKER=1`, `4T` | **433** | **144.0 FPS** | **83.7 FPS** | **6.95 ms** | **1.83 / 2.89 ms** | **20** |
| **3. Balanced** | `LOWEND=0`, `PLAY_WORKER=1`, `4T` | **101** | **33.4 FPS** | **25.8 FPS** | **29.94 ms** | **2.12 / 3.17 ms** | **35** |
| **4. High / Perf** | `LOWEND=0`, `PLAY_WORKER=1`, `All` | **94** | **31.2 FPS** | **20.5 FPS** | **32.02 ms** | **2.45 / 3.65 ms** | **35** |

> Enabling the low-end profile (`RUSTUDIO_LOWEND=1`, `PLAY_WORKER=0`, `RAYON_NUM_THREADS=2`) delivers **~4.9× higher FPS** (**152.9 FPS** vs **31.2 FPS**) and cuts draw calls from `35` down to `20`.

---

## Key Features

### Performance & Hardware Tuning
- **Hardware Tuner (`crates/rustudio-core/src/hardware_tuner.rs`):** Auto-detects CPU/RAM and applies `RUSTUDIO_LOWEND=1`, `RUSTUDIO_PLAY_WORKER=0`, and `RAYON_NUM_THREADS=2` on weak devices (`<= 2` cores or `< 3.8 GB` RAM).
- **Eco / High Modes (`crates/rustudio-core/src/mobile.rs`):** Switchable **Eco (30 FPS, 0.7× render scale)** and **High (60 FPS, 1.0× scale)** presets.
- **v0.3.4 Engine Gains:** Incremental voxel lighting (~97% less CPU), ~40% faster gameplay step, multi-threaded World GUI layout, and off-thread image decoding.

### Coding, AI & Sync
- **Luau Script Editor (`src/script_editor.rs`):** Syntax highlighting, API/member autocomplete, and `--!native` codegen.
- **Output & Profilers (`src/output_panel.rs`, `Ctrl+F6`):** Real-time Server/Client/Core logs, MicroProfiler, and Script Profiler flame view.
- **MCP Server & AI Assistant (`src/mcp/`, `src/mcp_assistant.rs`):** Built-in MCP server (`127.0.0.1:44755`) + AI Assistant with `read_output`, Luau execution, and screenshot capture.
- **File Sync & Team Create (`src/file_sync/`):** Rojo / Argon project sync and live collaborative editing with terrain streaming.

### Mobile & Touch UX (Android / Waydroid)
- **Immersive Fullscreen (`android/java/com/rtrust/mobile/MainActivity.java`):** Hides status bar and Android navigation bar automatically.
- **Responsive Dual Layout (`src/studio_ribbon.rs`):** Adaptive Mobile Drawers (`3D | Code | Explorer | Output`) + floating shortcut bar.
- **Touch Explorer (`src/explorer.rs`):** Always-visible `+` (Insert Object) button with 24px touch target; accidental double-tap rename disabled (rename via Properties).
- **3D Workspace Camera (`src/edit_camera.rs`):** Virtual Fly Joystick, `▲/▼` altitude buttons, `🎯 Focus`, `Fly / Orbit` toggle, `👁 Cam` free-look mode, and marquee box-select disabled on mobile.
- **Play Mode Controls (`src/viewport.rs`, `crates/rustudio-player/src/play.rs`):** Virtual thumbstick, `JUMP` button, 360° full-screen touch-drag camera rotation, and **Rust Play Fallback** (built-in movement & 12.5-stud orbit camera when offline).
- **Android Storage & DNS (`crates/rustudio-core/src/paths.rs`, `crates/rustudio-account/src/android_net.rs`):** App-private writable data/cookie storage (`internal_data_path()`), `RootCerts::WebPki`, and direct UDP DNS fallback (`AndroidDnsResolver`).

---

## Download & Install

1. Download **`RtRustMobile-0.3.4-debug.apk`** from the **[v0.3.4-mobile-demo.1 Pre-release](https://github.com/4sakenme/RtRust/releases/tag/v0.3.4-mobile-demo.1)**.
2. Install on Android or Waydroid:
   ```bash
   # Android (via ADB)
   adb install -r RtRustMobile-0.3.4-debug.apk

   # Waydroid
   waydroid app install RtRustMobile-0.3.4-debug.apk
   ```

---

## Build from Source

```bash
# Desktop (Linux)
cargo build --release

# Android APK (requires Android NDK 26+, cargo-ndk, Gradle 8.10+)
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/26.3.11579264"

cargo ndk -P 26 -t arm64-v8a -t x86_64 -o android/jniLibs build --profile mobile-release --lib
cd android && gradle assembleDebug
```

---

## Credits & License

- **Upstream Engine:** Built on **[rippelz/rustudio](https://github.com/rippelz/rustudio)** (`v0.3.4`).
- **Disclaimer:** Independent open-source project; not affiliated with or endorsed by Roblox Corporation.
- **License:** `MIT OR Apache-2.0` (vendored crates in `vendor/` keep their respective licenses).
