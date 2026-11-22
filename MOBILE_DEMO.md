# RtRustMobile — Demo 1 (RuStudio Engine v0.3.4)

Phiên bản **RtRustMobile** hợp nhất toàn bộ engine **RuStudio v0.3.4** cùng giao diện & điều khiển tối ưu cho màn hình cảm ứng (Android / Waydroid) và laptop cấu hình yếu.

---

## 1. Benchmark Hiệu Năng (Play Mode — `dist/rustudio v0.3.4`)

Đo thực tế bằng `--demo --play --headless --no-ui --bench --seconds 3` qua 4 cấu hình từ thấp đến cao:

| Mức cấu hình | Thiết lập (`env`) | Frames (3s) | Avg FPS | 1% Low FPS | Avg Frame | Work / CPU | Draw calls |
|---|---|---:|---:|---:|---:|---:|---:|
| **1. Cực yếu (`Ultra-Low`)** | `LOWEND=1`, `PLAY_WORKER=0`, `2T` | **459** | **152.9 FPS** | **96.2 FPS** | **6.54 ms** | **1.40 / 2.26 ms** | **20** |
| **2. Máy yếu (`Low-End`)** | `LOWEND=1`, `PLAY_WORKER=1`, `4T` | **433** | **144.0 FPS** | **83.7 FPS** | **6.95 ms** | **1.83 / 2.89 ms** | **20** |
| **3. Cân bằng (`Balanced`)** | `LOWEND=0`, `PLAY_WORKER=1`, `4T` | **101** | **33.4 FPS** | **25.8 FPS** | **29.94 ms** | **2.12 / 3.17 ms** | **35** |
| **4. Cao (`High / Perf`)** | `LOWEND=0`, `PLAY_WORKER=1`, `All` | **94** | **31.2 FPS** | **20.5 FPS** | **32.02 ms** | **2.45 / 3.65 ms** | **35** |

- Khi bật tối ưu máy yếu (`RUSTUDIO_LOWEND=1` + `PLAY_WORKER=0` + `RAYON_NUM_THREADS=2`), hiệu năng tăng **~4.9×** (**152.9 FPS** so với **31.2 FPS**), `draw_calls` giảm từ 35 xuống 20, `batches` giảm từ 22 xuống 7.

---

## 2. Tối Ưu Hiệu Năng Đã Tích Hợp

- **Engine RuStudio v0.3.4**:
  - **Incremental Voxel Lighting**: giảm ~97% thời gian CPU tính ánh sáng trên map lớn.
  - **Gameplay Step tối ưu**: giảm ~40% chi phí mỗi bước gameplay và không bỏ qua frame hiển thị.
  - **Đa luồng World GUI & Giải mã ảnh ngoài Main Thread**: giảm giật khựng khi mở place hoặc render UI 3D.
  - **Mở Place nhanh 2–3×**, giảm CPU lúc rảnh, chọn nhiều object nhanh, hỗ trợ GPU/OpenGL cũ.
  - **Automatic Graphics Quality** + shader path nhẹ (`RUSTUDIO_LOWEND=1`), cache GUI/text layout, culled terrain water, cached static sun shadows.
- **Hardware Tuner (`crates/rustudio-core/src/hardware_tuner.rs`)**:
  - Tự nhận diện phần cứng (`BatterySaver`, `Balanced`, `Performance`) và áp dụng ngay khi khởi động (`RUSTUDIO_LOWEND=1`, `RUSTUDIO_PLAY_WORKER=0`, `RAYON_NUM_THREADS=2` cho máy yếu <= 2 nhân / < 3.8 GB RAM).
  - Chế độ **Eco (30 FPS)** / **High (60 FPS)** trong `crates/rustudio-core/src/mobile.rs` + render scale `0.7x` ở chế độ Eco.

---

## 3. Các Tính Năng Hỗ Trợ Code & Phát Triển

### Lập trình & Debug Luau
- **Script Editor (`src/script_editor.rs`)**: soạn thảo Luau có tô màu cú pháp, gợi ý tự động (autocomplete) API/member, hỗ trợ `--!native` native codegen.
- **Output Panel (`src/output_panel.rs`)**: xem log realtime từ Server/Client/Core, lọc log theo ngữ cảnh và tìm kiếm nhanh.
- **MicroProfiler (`Ctrl+F6`) & Script Profiler**: đo thời gian từng hàm Luau, call tree và flame view (`crates/rustudio-profiler-ui`).
- **Command Bar & Live Server Eval (`eval_server_luau`)**: chạy lệnh Luau trực tiếp trong lúc Playtest.

### AI & Đồng bộ Code Ngoài
- **MCP Server tích hợp (`127.0.0.1:44755`, `src/mcp/`)**: cho phép AI Agent ngoài đọc/chỉnh sửa place, chạy Luau, bật Play, đọc Output và chụp ảnh màn hình.
- **AI MCP Assistant (`src/mcp_assistant.rs`)**: tích hợp công cụ `read_output` và cấu hình **RuStudio Official MCP**.
- **File Sync (`src/file_sync/`)**: gắn trực tiếp project **Rojo / Argon** vào place đang mở (hỗ trợ cả Team Create).

### Giao Diện & Điều Khiển Cảm Ứng (Mobile / Waydroid)
- **Immersive Fullscreen (`android/java/com/rtrust/mobile/MainActivity.java`)**: ẩn thanh trạng thái (giờ/pin) và thanh điều hướng Android.
- **Dual Layout (`src/studio_ribbon.rs`)**: tự chuyển giữa Ribbon đầy đủ và **Mobile Drawers** (tab `3D | Code | Explorer | Output` + thanh shortcut nổi).
- **Explorer thân thiện cảm ứng (`src/explorer.rs`)**: nút `+` (Insert Object) luôn hiện không cần hover, mở rộng vùng bấm `24px`, tắt double-tap rename tránh sửa nhầm tên (chỉnh tên qua Properties).
- **3D Workspace Camera (`src/edit_camera.rs`)**: Virtual Fly Joystick, nút lên/xuống `▲/▼`, `🎯 Focus`, chuyển `🕊 Fly / 🔄 Orbit`, thanh chỉnh tốc độ, nút `👁 Cam` free-look và tắt kéo bôi đen (marquee) trên mobile.
- **Play Mode Mobile (`src/viewport.rs`, `crates/rustudio-player/src/play.rs`)**:
  - Thumbstick ảo + nút `JUMP` + kéo ngón tay trực tiếp trên vùng trống màn hình để xoay camera 360° như Roblox Mobile.
  - **Rust Play Fallback**: tự điều khiển nhân vật & camera third-person orbit (12.5 studs) ngay cả khi chưa kịp tải `PlayerModule` (offline).
- **Mạng & Lưu trữ Android (`crates/rustudio-core/src/paths.rs`, `crates/rustudio-account/src/android_net.rs`)**:
  - Lưu dữ liệu & cookie vào `internal_data_path()` của app (tránh lỗi Read-only filesystem).
  - Tích hợp `RootCerts::WebPki` + `AndroidDnsResolver` (UDP DNS fallback) cho Sign In, tải Asset và `Studio content`.
- **Font Manager (`src/font_manager.rs`)**: hỗ trợ tuỳ chỉnh/nhập font Code & UI.

---

## 4. Hướng Dẫn Build APK (Android)

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/26.3.11579264"

# Build thư viện .so cho arm64-v8a và x86_64 (API 26+)
cargo ndk -P 26 -t arm64-v8a -t x86_64 -o android/jniLibs build --profile mobile-release --lib

# Đóng gói APK Debug
cd android && gradle assembleDebug
```
