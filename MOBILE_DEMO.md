# RtRustMobile 0.3.4 (Pre-release)

**RtRustMobile** brings the **RuStudio v0.3.4** engine to Android and low-end devices with touch-first studio controls, immersive fullscreen UI, and aggressive hardware performance tuning.

---

## 1. Performance Benchmarks (Play Mode — `v0.3.4`)

Measured with `--demo --play --headless --no-ui --bench --seconds 3` across 4 hardware profiles from ultra-low to high-end:

| Profile Tier | Environment Config | Frames (3s) | Avg FPS | 1% Low FPS | Avg Frame | Work / CPU | Draw Calls |
|---|---|---:|---:|---:|---:|---:|---:|
| **1. Ultra-Low** | `LOWEND=1`, `PLAY_WORKER=0`, `2T` | **459** | **152.9 FPS** | **96.2 FPS** | **6.54 ms** | **1.40 / 2.26 ms** | **20** |
| **2. Low-End** | `LOWEND=1`, `PLAY_WORKER=1`, `4T` | **433** | **144.0 FPS** | **83.7 FPS** | **6.95 ms** | **1.83 / 2.89 ms** | **20** |
| **3. Balanced** | `LOWEND=0`, `PLAY_WORKER=1`, `4T` | **101** | **33.4 FPS** | **25.8 FPS** | **29.94 ms** | **2.12 / 3.17 ms** | **35** |
| **4. High / Perf** | `LOWEND=0`, `PLAY_WORKER=1`, `All` | **94** | **31.2 FPS** | **20.5 FPS** | **32.02 ms** | **2.45 / 3.65 ms** | **35** |

- Enabling the low-end profile (`RUSTUDIO_LOWEND=1`, `PLAY_WORKER=0`, `RAYON_NUM_THREADS=2`) delivers **~4.9× higher FPS** (**152.9 FPS** vs **31.2 FPS**), cuts `draw_calls` from 35 to 20, and cuts batches from 22 to 7.

---

## 2. Performance Optimizations

- **RuStudio v0.3.4 Engine Upgrades**:
  - **Incremental Voxel Lighting**: ~97% less lighting CPU time on large places.
  - **Optimized Gameplay Step**: ~40% lower CPU cost per gameplay step without skipping displayed frames.
  - **Multi-threaded World GUI & Off-thread Image Decoding**: eliminates main-thread hitches when loading places or rendering 3D UI.
  - **2–3× Faster Place Opening**, lower idle CPU usage, faster large selections, and startup fixes for older OpenGL/iGPUs.
  - **Automatic Graphics Quality**: lightweight low-end shader path (`RUSTUDIO_LOWEND=1`), cached GUI/text layout, culled terrain water, and cached static sun shadows.
- **Hardware Tuner (`crates/rustudio-core/src/hardware_tuner.rs`)**:
  - Automatically detects system hardware (`BatterySaver`, `Balanced`, `Performance`) and applies `RUSTUDIO_LOWEND=1`, `RUSTUDIO_PLAY_WORKER=0`, and `RAYON_NUM_THREADS=2` on weak devices (`<= 2` cores or `< 3.8 GB` RAM).
  - Supports **Eco (30 FPS, 0.7× render scale)** and **High (60 FPS, 1.0× scale)** modes (`crates/rustudio-core/src/mobile.rs`).

---

## 3. Coding & Development Features

### Luau Scripting & Debugging
- **Script Editor (`src/script_editor.rs`)**: Luau code editor with syntax highlighting, API/member autocomplete, and `--!native` codegen support.
- **Output Panel (`src/output_panel.rs`)**: real-time Server/Client/Core logs with context filtering and search.
- **MicroProfiler (`Ctrl+F6`) & Script Profiler**: per-function Luau timing, call tree, and flame graph (`crates/rustudio-profiler-ui`).
- **Live Server Evaluation (`eval_server_luau`)**: run Luau commands directly inside the live Playtest server VM.

### AI Integration & External Sync
- **Built-in MCP Server (`127.0.0.1:44755`, `src/mcp/`)**: allows external AI agents to read/edit places, execute Luau, toggle Play, read Output, and capture screenshots.
- **AI MCP Assistant (`src/mcp_assistant.rs`)**: includes `read_output` tooling and RuStudio 0.3.3+ Official MCP support.
- **File Sync (`src/file_sync/`)**: attach **Rojo** or **Argon** projects directly to any open place (including Team Create sessions).

### Mobile & Touch UX (Android / Waydroid)
- **Immersive Fullscreen (`android/java/com/rtrust/mobile/MainActivity.java`)**: hides Android status bar and navigation bar for edge-to-edge workspace editing.
- **Responsive Dual Layout (`src/studio_ribbon.rs`)**: switches between full Desktop Ribbon and **Mobile Drawers** (`3D | Code | Explorer | Output` + floating shortcut bar).
- **Touch-friendly Explorer (`src/explorer.rs`)**: always-visible `+` (Insert Object) button with expanded 24px hit area, and double-tap rename disabled on touch (rename via Properties).
- **3D Workspace Camera (`src/edit_camera.rs`)**: Virtual Fly Joystick, `▲/▼` altitude controls, `🎯 Focus`, `🕊 Fly / 🔄 Orbit` toggle, speed slider, `👁 Cam` free-look toggle, and marquee box-select disabled on mobile.
- **Mobile Play Controls (`src/viewport.rs`, `crates/rustudio-player/src/play.rs`)**:
  - Virtual thumbstick + `JUMP` button + full-screen touch-drag 360° camera rotation (drag anywhere outside controls, just like Roblox Mobile).
  - **Rust Play Fallback**: built-in character movement and third-person orbit camera (12.5 studs) when offline or before `PlayerModule` finishes downloading.
- **Android Storage & Networking (`crates/rustudio-core/src/paths.rs`, `crates/rustudio-account/src/android_net.rs`)**:
  - Routes app data, settings, and session cookies to `internal_data_path()` (fixing read-only filesystem errors).
  - Uses `RootCerts::WebPki` and a custom `AndroidDnsResolver` (direct UDP DNS fallback) for Sign In, asset downloads, and Studio content fetching.
- **Font Manager (`src/font_manager.rs`)**: custom Code & UI font importing.

---

## 4. Building from Source (Android APK)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/26.3.11579264"

# Build native libraries for arm64-v8a and x86_64 (API 26+)
cargo ndk -P 26 -t arm64-v8a -t x86_64 -o android/jniLibs build --profile mobile-release --lib

# Assemble Debug APK
cd android && gradle assembleDebug
```
