mod api;
mod config;
mod fflags;
mod gui;
mod hardware;
mod installer;
mod ipc;
mod prefix;
mod runtime;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::LauncherConfig;
use hardware::HardwareProfile;
use installer::Installer;
#[cfg(unix)]
use ipc::IpcServer;
use prefix::PrefixManager;
use std::process::Command;
use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "studio-launcher",
    about = "Ultra-lightweight Roblox Studio runner and manager for Linux & Windows",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct authentication or protocol URI (e.g. roblox-studio://...)
    #[arg(short, long)]
    uri: Option<String>,

    /// Target framerate limit
    #[arg(long)]
    fps: Option<u32>,

    /// Run in graphical launcher UI mode (default when no command is specified)
    #[arg(long)]
    gui: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launches Roblox Studio with optimal settings (headless / CLI mode)
    Run {
        #[arg(short, long)]
        place_id: Option<u64>,
    },
    /// Updates or pre-downloads the latest Roblox Studio version
    Update,
    /// Inspects system hardware, Wine environment, and graphics stack
    Doctor,
    /// Opens the graphical Game Launcher UI
    Gui,
}

fn main() -> Result<()> {
    // Ensure clean UTF-8 locale to prevent Gtk-WARNING
    if std::env::var_os("LC_ALL").is_none() {
        std::env::set_var("LC_ALL", "C.UTF-8");
    }
    // Prevent WebKitGTK DMABUF renderer issues on Linux Intel/Mesa Wayland
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // If no command specified or `--gui` passed, launch the Game Launcher GUI!
    match cli.command {
        None | Some(Commands::Gui) => {
            launch_gui()?;
            return Ok(());
        }
        Some(Commands::Run { place_id }) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                run_studio_cli(place_id, cli.fps).await
            })?;
        }
        Some(Commands::Update) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                update_studio().await
            })?;
        }
        Some(Commands::Doctor) => {
            run_doctor()?;
        }
    }

    Ok(())
}

fn launch_gui() -> Result<()> {
    gui::run_gui()
}

async fn run_studio_cli(place_id: Option<u64>, target_fps: Option<u32>) -> Result<()> {
    let mut config = LauncherConfig::load();
    if let Some(fps) = target_fps {
        config.target_fps = fps;
    }
    if place_id.is_some() {
        config.place_id = place_id;
    }

    let profile = HardwareProfile::detect();
    let prefix_mgr = Arc::new(PrefixManager::new()?);

    #[cfg(target_os = "windows")]
    {
        let studio_exe = if let Some(existing) = Installer::find_existing_studio_windows() {
            info!("Found existing Windows Roblox Studio: {:?}", existing);
            let version_dir = existing.parent().context("Invalid studio exe directory")?;
            let _ = crate::fflags::FFlagsGenerator::inject_settings(version_dir, &config);
            existing
        } else {
            let installer = Installer::new(prefix_mgr.clone());
            installer.ensure_latest_studio(&profile, &config).await?
        };

        let studio_dir = studio_exe.parent().context("Invalid studio exe path")?;
        let potato = config.profile_mode == "potato" || config.block_webview2;
        Installer::safe_swap_webview2(studio_dir, potato)?;

        info!("Starting Roblox Studio (Native Windows)...");
        let mut cmd = Command::new(&studio_exe);
        cmd.current_dir(studio_dir);
        cmd.arg("-ide");

        if let Some(id) = config.place_id {
            cmd.args(["-placeId", &id.to_string()]);
        }

        let status = cmd
            .status()
            .context("Failed to execute native RobloxStudioBeta.exe")?;

        info!("Roblox Studio exited with status: {}", status);
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Setup prefix (registry overrides, dxvk)
        prefix_mgr.setup_prefix(&config.mouse_warp_mode)?;

        // Download / verify latest studio release
        let installer = Installer::new(prefix_mgr.clone());
        let studio_exe = installer
            .ensure_latest_studio(&profile, &config)
            .await?;

        // Spawn IPC server in background to handle web redirects
        tokio::spawn(async move {
            let server = IpcServer::new();
            if let Err(e) = server.run().await {
                error!("IPC server encountered error: {}", e);
            }
        });

        let cache_dir = prefix_mgr.data_dir.join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);

        info!("Starting Roblox Studio under Wine...");
        let studio_dir = studio_exe.parent().context("Invalid studio exe path")?;
        let mut cmd = Command::new("wine");
        cmd.current_dir(studio_dir);
        cmd.env("WINEPREFIX", &prefix_mgr.prefix_dir);
        cmd.env("WINEDEBUG", "-all");
        cmd.env("WINEFSYNC", "1");
        cmd.env("WINEESYNC", "1");
        cmd.env("DXVK_HUD", "0");
        cmd.env("DXVK_STATE_CACHE", "1");
        cmd.env("DXVK_STATE_CACHE_PATH", &cache_dir);
        cmd.env("ANV_ENABLE_PIPELINE_CACHE", "1");
        cmd.env("MESA_VK_WSI_PRESENT_MODE", "mailbox");
        cmd.env("QT_QPA_PLATFORM_PLUGIN_PATH", "Qt5/platforms");

        // Wayland camera fix
        if config.force_xwayland_mouse {
            cmd.env_remove("WAYLAND_DISPLAY");
        }

        if config.virtual_desktop {
            cmd.args(["explorer", &format!("/desktop=Studio,{}", config.virtual_desktop_res)]);
        }

        cmd.arg(&studio_exe);
        cmd.arg("-ide");

        if let Some(id) = config.place_id {
            cmd.args(["-placeId", &id.to_string()]);
        }

        let status = cmd
            .status()
            .context("Failed to execute Wine process. Ensure 'wine' is installed on your PATH.")?;

        info!("Roblox Studio exited with status: {}", status);
        Ok(())
    }
}

async fn update_studio() -> Result<()> {
    let profile = HardwareProfile::detect();
    let config = LauncherConfig::load();
    let prefix_mgr = Arc::new(PrefixManager::new()?);
    let installer = Installer::new(prefix_mgr);

    installer.ensure_latest_studio(&profile, &config).await?;
    info!("Update check finished successfully.");
    Ok(())
}

fn run_doctor() -> Result<()> {
    #[cfg(target_os = "windows")]
    println!("=== System Diagnostics for Roblox Studio on Windows ===");
    #[cfg(not(target_os = "windows"))]
    println!("=== System Diagnostics for Roblox Studio on Linux ===");

    // 1. Hardware Profile
    let profile = HardwareProfile::detect();
    println!("RAM: {} MiB", profile.total_ram_mib);
    println!("GPU Vendor: {:?}", profile.primary_gpu);
    println!("Integrated GPU: {}", profile.is_integrated_gpu);
    println!("CPU Threads: {}", profile.cpu_cores);
    println!(
        "Recommended DXVK Threads: {}",
        profile.recommended_dxvk_compiler_threads()
    );

    #[cfg(target_os = "windows")]
    {
        if let Some(exe) = Installer::find_existing_studio_windows() {
            println!("Native Roblox Studio: FOUND at {:?}", exe);
        } else {
            println!("Native Roblox Studio: NOT DETECTED (Launcher will auto-install on demand)");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 2. Display Server
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        let x11 = std::env::var("DISPLAY").ok();
        println!(
            "Display Server: {}",
            if wayland.is_some() {
                "Wayland"
            } else if x11.is_some() {
                "X11"
            } else {
                "Unknown"
            }
        );

        // 3. Wine availability
        let wine_check = Command::new("wine").arg("--version").output();
        match wine_check {
            Ok(out) => println!("Wine: {}", String::from_utf8_lossy(&out.stdout).trim()),
            Err(_) => println!("Wine: NOT FOUND (Please install Wine or Proton)"),
        }
    }

    Ok(())
}
