<p align="center">
  <img src="assets/icons/rtrust.svg" width="128" alt="RtRustMobile icon">
</p>

<h1 align="center">RtRustMobile (v0.5.1)</h1>

<p align="center">
  <strong>Modern, High-Performance Roblox Studio Editor &amp; Runtime in Rust for iOS, Android, Waydroid &amp; Desktop</strong>
</p>

<p align="center">
  <a href="https://github.com/4sakenme/RtRust/releases"><img src="https://img.shields.io/badge/Release-v0.5.1-FF5A26?style=flat-square" alt="Release v0.5.1"></a>
  <a href="https://github.com/rippelz/rustudio"><img src="https://img.shields.io/badge/Engine-rustudio%20v0.4.0+-26C6DA?style=flat-square" alt="Engine"></a>
  <img src="https://img.shields.io/badge/UI-Stitch%20Obsidian%20Design-FF5A26?style=flat-square" alt="UI Design">
  <img src="https://img.shields.io/badge/Platform-iOS%20%7C%20Android%20%7C%20Linux%20%7C%20Waydroid-3DDC84?style=flat-square" alt="Platforms">
  <img src="https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-lightgrey?style=flat-square" alt="License">
</p>

---

## Overview

**RtRustMobile** is a sleek, native Rust Roblox Studio editor and runtime designed specifically for mobile (**iOS** & **Android**) and resource-constrained devices. Powered by the **[rustudio](https://github.com/rippelz/rustudio)** engine (no Wine, no JVM simulation), it pairs a modern **Obsidian & Rust-Orange Studio UI** with a low-memory streaming architecture capable of loading and playtesting **heavy, complex places** smoothly on mobile hardware.

- **Complete Studio Toolset:** Native Luau VM (`--!native` codegen), Rapier3D & Studio physics, R6/R15 character simulation, `wgpu` (Metal / Vulkan) rendering, incremental voxel lighting, Team Create live sync, Rojo/Argon File Sync, MicroProfiler, and built-in MCP server.
- **Built for Heavy Places & Touch Workflows:** Engineered with progressive asset streaming, GPU buffer deduplication, and touch-first workspace controls so large maps stay responsive and well within mobile memory limits.

---

## Modern UI / UX Design 

Crafted with a clean dark `#111318` Obsidian canvas, crisp typography, and vibrant `#FF5A26` Rust-Orange + `#26C6DA` Cyan accents for an intuitive mobile & desktop creation experience:

- **Animated Spatial Loading Screen (`src/loading_screen.rs`):** Sleek `#0B0D12` dark canvas featuring a rotating Rust-orange gear, breathing cyan Studio cube, 3D gizmo axes, hardware backend badge (`Metal · 64-bit` / `Vulkan · 64-bit`), and a frosted-glass progress pill.
- **Redesigned Home & Templates Hub (`src/start_page.rs`):** Responsive Portrait & Landscape start screen with quick actions (`+ New Baseplate`, `</> Focus Code IDE`, `Open .rbxl File`, `Studio Settings`), recent places, and categorized 3D templates.
- **Focus Code IDE Mode (`src/studio_ribbon.rs`):** Distraction-free Luau scripting workspace (`viewport_hidden = true`) that suspends 3D scene rendering to maximize screen space, battery life, and RAM.
- **Draggable 3-Side Explorer + Corner Resize:** Long-press & drag `≡ Explorer` to preview a translucent snap ghost and dock cleanly to the **Left**, **Right**, or **Bottom**, complete with a `◢` corner grip for fluid resizing.
- **Vertically Resizable Output Drawer:** Smooth full-width drag handle (`↕`) on the Output console across Desktop, Landscape Mobile, and Focus Code IDE.
- **Touch-Friendly 3D Viewport Actions (`src/viewport_menu.rs`):** Long-press (`≥ 0.48s`) any Part in the 3D Viewport to trigger a floating quick-action card (`Duplicate (Clone)`, `Copy`, `Paste`, `Focus`, `Group`, `Delete`).

---

## Memory & Performance Architecture (Heavy Places on Mobile)

Heavy places with hundreds of `MeshPart`s and high-resolution textures can quickly hit strict OS memory ceilings (such as iOS's **2098 MB ActiveHard Jetsam limit** on 4 GB devices) if assets decode all at once. `RtRustMobile v0.5.1` keeps memory usage low and stable through 6 coordinated optimizations:

1. **Immediate CPU Buffer Drop (`crates/rustudio-core/src/assets.rs`):**
   - Raw decoded pixel `Vec<u8>` and vertex/normal/UV buffers are freed immediately after `wgpu::Texture` / `wgpu::Buffer` upload, and compressed `AssetStore::memory` bytes are evicted right after GPU upload on mobile.
2. **AssetId `Weak`/`Arc` Deduplication (`MeshCache` & `TextureCache`):**
   - Repeated `MeshPart`s and decals sharing the same `AssetId` share a single GPU allocation via `HashMap<u64, Weak<LoadedMesh / LoadedTexture>>`.
3. **Progressive Frame-Budgeted Streaming (`≤ 4ms` & Max 2 Assets/Frame):**
   - Prevents memory spikes and frame freezes when opening or playtesting heavy places by capping decode + GPU upload to **at most 2 assets per frame** within a **4.0 ms** frame budget.
4. **Spatial Ring Priority (`0–250 studs` Load, `> 350 studs` Unload):**
   - `crates/rustudio-player/src/game_view.rs` prioritizes parts closest to the camera (`0–120 studs` first), renders distant parts (`> 250 studs`) as lightweight colored proxy boxes until approached, and unloads geometry beyond `350 studs`.
5. **Staggered Playtest Boot (`Max 3 Scripts/Frame`) & Lazy Audio:**
   - `crates/rustudio-player/src/play.rs` initializes `Script` / `LocalScript` instances gradually (`3` per frame) so Playtest starts smoothly without UI hangs.
   - `crates/rustudio-engine/src/domains/media.rs` defers decoding `.ogg`/`.mp3` audio into raw PCM on iOS until `.Play()` is invoked.
6. **Mobile Texture Cap, `160 MB` LRU Cache & Low-Memory Purge:**
   - `crates/rustudio-core/src/texture.rs` downscales iOS textures to **`512×512`** (`256×256` in **Eco** mode) without cloning full pixel buffers.
   - `AssetStore::memory` enforces a strict **`160 MB` (iOS) / `200 MB` (Android)** LRU cache limit.
   - `src/platform.rs` listens to `UIApplicationDidReceiveMemoryWarningNotification` to pause outer-ring streaming, purge unused asset caches, and trigger `lua.gc_collect()`.

---

## Performance Benchmarks

Measured with `--play --headless --no-ui --bench --seconds 3`:

| Place / Tier | Environment Config | Frames (3s) | Avg FPS | 1% Low FPS | Avg Frame | Work / CPU | Draw Calls |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| **Demo — Ultra-Low** | `LOWEND=1`, `PLAY_WORKER=0`, `2T` | **459** | **152.9 FPS** | **96.2 FPS** | **6.54 ms** | **1.40 / 2.26 ms** | **20** |
| **Demo — Low-End** | `LOWEND=1`, `PLAY_WORKER=1`, `4T` | **433** | **144.0 FPS** | **83.7 FPS** | **6.95 ms** | **1.83 / 2.89 ms** | **20** |
| **Demo — Balanced** | `LOWEND=0`, `PLAY_WORKER=1`, `4T` | **101** | **33.4 FPS** | **25.8 FPS** | **29.94 ms** | **2.12 / 3.17 ms** | **35** |
| **Demo — High** | `LOWEND=0`, `PLAY_WORKER=1`, `All` | **94** | **31.2 FPS** | **20.5 FPS** | **32.02 ms** | **2.45 / 3.65 ms** | **35** |
| **`aaaa.rbxl` — Ultra-Low** | `LOWEND=1`, `PLAY_WORKER=0`, `2T` | **1795** | — | — | — | — | **18** |
| **`aaaa.rbxl` — High** | `LOWEND=0`, `PLAY_WORKER=1`, `All` | **1621** | — | — | — | — | **18** |

---

## Build from Source

```bash
# Desktop (Linux / macOS / Windows)
cargo build --release

# iOS (arm64-apple-ios)
cargo build --release --target aarch64-apple-ios

# Android APK (arm64-v8a)
cargo ndk -t arm64-v8a --platform 26 -o android/jniLibs build --release --lib
gradle -p android assembleDebug
