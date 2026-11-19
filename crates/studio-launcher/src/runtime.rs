//! Slim-Wine Runtime & Linux cgroups v2 / DXGI Shim Manager.
//!
//! Controls system resources (CPU, RAM, VRAM, GPU) when running Roblox Studio
//! under Wine, and integrates the Rust DXGI Proxy Shim (`dxgi.dll`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[allow(unused_imports)]
use tracing::{debug, info, warn};

pub const STUB_DXGI_BYTES: &[u8] = include_bytes!("../../../assets/dll/dxgi.dll");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuTelemetry {
    pub fps: u32,
    pub vram_cap_mb: u32,
    pub alt_tab_throttled: bool,
}

pub struct SlimRuntimeManager;

impl SlimRuntimeManager {
    /// Detects portable slim Wine, or falls back to system Wine from PATH.
    #[allow(dead_code)]
    pub fn get_wine_binary() -> PathBuf {
        crate::prefix::PrefixManager::get_wine_binary()
    }

    /// Prepares an isolated slim Wine prefix with unnecessary background services disabled.
    #[allow(dead_code)]
    pub fn setup_slim_prefix(prefix_dir: &Path, mouse_warp: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let _ = (prefix_dir, mouse_warp);
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            info!("Configuring isolated Slim Wine prefix at {:?}", prefix_dir);
            let wine_bin = Self::get_wine_binary();

            // 1. Disable redundant Wine background services in Registry (Start = 4: Disabled)
            // Saves memory and eliminates unnecessary background polling threads.
            let redundant_services = [
                "Spooler",      // Print Spooler service
                "LanmanServer", // Network file sharing
                "BITS",         // Background Intelligent Transfer Service
                "RpcLocator",   // RPC locator service
            ];

            for svc in &redundant_services {
                let _ = Command::new(&wine_bin)
                    .env("WINEPREFIX", prefix_dir)
                    .env("WINEDEBUG", "-all")
                    .args([
                        "reg",
                        "add",
                        &format!("HKLM\\System\\CurrentControlSet\\Services\\{}", svc),
                        "/v",
                        "Start",
                        "/t",
                        "REG_DWORD",
                        "/d",
                        "4",
                        "/f",
                    ])
                    .status();
            }

            // 2. DirectInput & X11 Driver mouse settings for Wayland
            let _ = Command::new(&wine_bin)
                .env("WINEPREFIX", prefix_dir)
                .env("WINEDEBUG", "-all")
                .args([
                    "reg",
                    "add",
                    "HKCU\\Software\\Wine\\DirectInput",
                    "/v",
                    "MouseWarpOverride",
                    "/t",
                    "REG_SZ",
                    "/d",
                    mouse_warp,
                    "/f",
                ])
                .status();

            let _ = Command::new(&wine_bin)
                .env("WINEPREFIX", prefix_dir)
                .env("WINEDEBUG", "-all")
                .args([
                    "reg",
                    "add",
                    "HKCU\\Software\\Wine\\X11 Driver",
                    "/v",
                    "GrabPointer",
                    "/t",
                    "REG_SZ",
                    "/d",
                    "Y",
                    "/f",
                ])
                .status();

            let _ = Command::new(&wine_bin)
                .env("WINEPREFIX", prefix_dir)
                .env("WINEDEBUG", "-all")
                .args([
                    "reg",
                    "add",
                    "HKCU\\Software\\Wine\\X11 Driver",
                    "/v",
                    "UseCapture",
                    "/t",
                    "REG_SZ",
                    "/d",
                    "Y",
                    "/f",
                ])
                .status();

            // 3. DLL overrides: enforce native DXVK / DXGI shim and disable bloat
            let native_dlls = [
                "dxgi",
                "d3d11",
                "WebView2Loader",
                "concrt140",
                "msvcp140",
                "msvcp140_1",
                "msvcp140_2",
                "ucrtbase",
                "vcomp140",
                "vcruntime140",
                "vcruntime140_1",
            ];

            for dll in &native_dlls {
                let _ = Command::new(&wine_bin)
                    .env("WINEPREFIX", prefix_dir)
                    .env("WINEDEBUG", "-all")
                    .args([
                        "reg",
                        "add",
                        "HKCU\\Software\\Wine\\DllOverrides",
                        "/v",
                        dll,
                        "/t",
                        "REG_SZ",
                        "/d",
                        "native,builtin",
                        "/f",
                    ])
                    .status();
            }

            // Explicitly disable winemenubuilder (prevents creating unwanted host .desktop files)
            let disabled_dlls = ["winemenubuilder.exe"];
            for dll in &disabled_dlls {
                let _ = Command::new(&wine_bin)
                    .env("WINEPREFIX", prefix_dir)
                    .env("WINEDEBUG", "-all")
                    .args([
                        "reg",
                        "add",
                        "HKCU\\Software\\Wine\\DllOverrides",
                        "/v",
                        dll,
                        "/t",
                        "REG_SZ",
                        "/d",
                        "",
                        "/f",
                    ])
                    .status();
            }

            // Ensure msedgewebview2.exe is NEVER disabled in DllOverrides (Studio Hub requires it)
            let _ = Command::new(&wine_bin)
                .env("WINEPREFIX", prefix_dir)
                .env("WINEDEBUG", "-all")
                .args([
                    "reg",
                    "delete",
                    "HKCU\\Software\\Wine\\DllOverrides",
                    "/v",
                    "msedgewebview2.exe",
                    "/f",
                ])
                .status();

            Ok(())
        }
    }

    /// Deploys or restores the Rust DXGI Proxy Shim (`dxgi.dll`) and forwards to `dxgi_vk.dll`.
    pub fn deploy_dxgi_shim(
        studio_dir: &Path,
        enable_shim: bool,
        alt_tab_throttle: bool,
        vram_cap_mb: u32,
    ) -> Result<()> {
        let dxgi_path = studio_dir.join("dxgi.dll");
        let dxgi_vk_path = studio_dir.join("dxgi_vk.dll");
        let config_path = studio_dir.join("rtrust_shim.json");

        if enable_shim {
            // If existing dxgi.dll is DXVK (not our shim), rename it to dxgi_vk.dll
            if dxgi_path.exists() {
                if let Ok(metadata) = fs::metadata(&dxgi_path) {
                    if metadata.len() != STUB_DXGI_BYTES.len() as u64 {
                        info!("Renaming original DXVK dxgi.dll -> dxgi_vk.dll for proxying");
                        let _ = fs::copy(&dxgi_path, &dxgi_vk_path);
                    }
                }
            }

            // Write our Rust dxgi.dll shim
            fs::write(&dxgi_path, STUB_DXGI_BYTES)
                .context("Failed to write Rust DXGI shim into studio directory")?;
            info!("Rust DXGI Proxy Shim deployed to {:?}", dxgi_path);

            // Write shim configuration file
            let config_json = serde_json::json!({
                "alt_tab_throttle": alt_tab_throttle,
                "alt_tab_sleep_ms": if alt_tab_throttle { 100 } else { 0 },
                "vram_cap_mb": vram_cap_mb
            });
            let _ = fs::write(&config_path, config_json.to_string());
        } else {
            // Restore original dxgi_vk.dll if present
            if dxgi_vk_path.exists() {
                info!("Restoring original DXVK dxgi_vk.dll -> dxgi.dll");
                let _ = fs::copy(&dxgi_vk_path, &dxgi_path);
                let _ = fs::remove_file(&dxgi_vk_path);
            } else if dxgi_path.exists() {
                // If it's our shim, delete it so Wine loads native DXVK from prefix system32
                if let Ok(metadata) = fs::metadata(&dxgi_path) {
                    if metadata.len() == STUB_DXGI_BYTES.len() as u64 {
                        info!("Removing Rust DXGI shim from studio dir");
                        let _ = fs::remove_file(&dxgi_path);
                    }
                }
            }
            if config_path.exists() {
                let _ = fs::remove_file(&config_path);
            }
        }

        Ok(())
    }

    /// Wraps a command inside a Linux cgroups v2 user scope using `systemd-run`
    /// to enforce strict RAM and CPU weight constraints.
    #[allow(dead_code, unused_variables)]
    pub fn build_launch_command(
        runner_bin: &Path,
        args: &[String],
        cgroup_ram_gb: u32,
        cpu_weight: u32,
    ) -> Command {
        #[cfg(target_os = "linux")]
        {
            if cgroup_ram_gb > 0 && is_systemd_run_available() {
                let scope_name = format!("rtrust-studio-{}", std::process::id());
                let memory_max = format!("{}G", cgroup_ram_gb);
                let memory_high = format!("{}G", (cgroup_ram_gb * 85) / 100);

                info!(
                    "Wrapping Studio with Linux cgroups v2: MemoryHigh={}, MemoryMax={}, CPUWeight={}",
                    memory_high, memory_max, cpu_weight
                );

                let mut cmd = Command::new("systemd-run");
                cmd.args([
                    "--user",
                    "--scope",
                    &format!("--unit={}", scope_name),
                    "-p",
                    &format!("MemoryHigh={}", memory_high),
                    "-p",
                    &format!("MemoryMax={}", memory_max),
                    "-p",
                    &format!("CPUWeight={}", cpu_weight),
                    "--description=RTRUST Roblox Studio Sandbox",
                ]);

                cmd.arg(runner_bin);
                cmd.args(args);
                return cmd;
            }
        }

        let mut cmd = Command::new(runner_bin);
        cmd.args(args);
        cmd
    }

    /// Reads live GPU telemetry written by `dxgi-shim`.
    pub fn read_gpu_telemetry() -> Option<GpuTelemetry> {
        let paths = [
            PathBuf::from("/tmp/rtrust_gpu_stats.json"),
            PathBuf::from("Z:\\tmp\\rtrust_gpu_stats.json"),
        ];

        for path in &paths {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(stats) = serde_json::from_str::<GpuTelemetry>(&data) {
                    return Some(stats);
                }
            }
        }

        None
    }
}

#[cfg(target_os = "linux")]
fn is_systemd_run_available() -> bool {
    Command::new("which")
        .arg("systemd-run")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
