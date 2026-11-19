# Credits & Acknowledgments

RtRust relies on and is inspired by the following open-source projects:

## 1. rustudio
- **Repository:** [https://github.com/rippelz/rustudio](https://github.com/rippelz/rustudio)
- **Author:** rippelz & contributors
- **License:** MIT / Apache-2.0
- **Contribution to RtRust:**
  - Architecture and endpoints for Roblox Cloud API authentication, Quick Login via cryptographic key pairs, and session redemption.
  - Multi-source Experience & Place discovery (Personal Universes, Group Universes, Shared/Team Create games, and sub-places).
  - High-resolution thumbnail batch querying and local disk caching strategy.
  - Studio launch mode conventions (Direct Cloud Edit via `roblox-studio:1+launchmode:edit...` and local `.rbxl` file loading).

## 2. Vinegar
- **Repository:** [https://github.com/vinegarhq/vinegar](https://github.com/vinegarhq/vinegar)
- **License:** GPL-3.0
- **Contribution to RtRust:**
  - Insights into Wine DirectInput registry configurations (`MouseWarpOverride=force`) and X11 driver window focus behavior for Roblox Studio under Linux.
  - Inspiration for zero-overhead native runtime optimizations without Flatpak container bottlenecks.

## 3. DXVK
- **Repository:** [https://github.com/doitsujin/dxvk](https://github.com/doitsujin/dxvk)
- **License:** Zlib
- **Contribution to RtRust:**
  - Vulkan-based Direct3D 11 translation layers enabling near-native graphics performance on Linux.

## 4. Tauri / Wry / Tao
- **Repository:** [https://github.com/tauri-apps/wry](https://github.com/tauri-apps/wry)
- **License:** MIT / Apache-2.0
- **Contribution to RtRust:**
  - Ultra-lightweight cross-platform webview rendering powered by system WebKitGTK on Linux.
