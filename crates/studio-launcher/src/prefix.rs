use crate::hardware::HardwareProfile;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

#[allow(dead_code)]
pub struct PrefixManager {
    pub prefix_dir: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl PrefixManager {
    /// Initializes standard XDG paths for data and configuration.
    pub fn new() -> Result<Self> {
        let base_data = dirs::data_local_dir()
            .context("Could not determine local data directory")?
            .join("rtrust");

        let base_config = dirs::config_dir()
            .context("Could not determine configuration directory")?
            .join("rtrust");

        let prefix_dir = base_data.join("prefix");

        fs::create_dir_all(&base_data)?;
        fs::create_dir_all(&base_config)?;
        fs::create_dir_all(&prefix_dir)?;

        Ok(Self {
            prefix_dir,
            config_dir: base_config,
            data_dir: base_data,
        })
    }

    /// Detects Vinegar's Kombucha Wine (which includes the XFixes cursor warp patch)
    /// or falls back to system Wine.
    pub fn get_wine_binary() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            // 1. Direct Vinegar kombucha symlink/path
            let direct = home.join(".var/app/org.vinegarhq.Vinegar/data/vinegar/kombucha/bin/wine");
            if direct.exists() {
                info!("Using Vinegar Kombucha Wine (with XFixes cursor warp fix): {:?}", direct);
                return direct;
            }

            // 2. Scan for kombucha-* folders in Vinegar data dir
            let vinegar_data = home.join(".var/app/org.vinegarhq.Vinegar/data/vinegar");
            if let Ok(entries) = fs::read_dir(&vinegar_data) {
                for entry in entries.flatten() {
                    let wine_candidate = entry.path().join("bin/wine");
                    if wine_candidate.exists() {
                        info!("Using Vinegar Kombucha Wine build: {:?}", wine_candidate);
                        return wine_candidate;
                    }
                }
            }

            // 3. Check ~/.local/share/rtrust/kombucha/bin/wine
            let local_kombucha = home.join(".local/share/rtrust/kombucha/bin/wine");
            if local_kombucha.exists() {
                info!("Using local Kombucha Wine: {:?}", local_kombucha);
                return local_kombucha;
            }
        }

        PathBuf::from("wine")
    }

    /// Resolves the Roblox Plugins directory inside the Wine prefix or native Windows user directory.
    pub fn plugins_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Some(local_app) = dirs::data_local_dir() {
                let dir = local_app.join("Roblox").join("Plugins");
                let _ = fs::create_dir_all(&dir);
                return dir;
            }
            return PathBuf::from("Plugins");
        }

        #[cfg(not(target_os = "windows"))]
        {
            let users_dir = self.prefix_dir.join("drive_c/users");
            let mut target_user = "steamuser".to_string();

            if let Ok(entries) = fs::read_dir(&users_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name != "Public" && name != "Default" && !name.is_empty() {
                        target_user = name;
                        break;
                    }
                }
            }

            let dir = users_dir
                .join(target_user)
                .join("AppData/Local/Roblox/Plugins");
            let _ = fs::create_dir_all(&dir);
            dir
        }
    }

    /// Opens the Studio Plugins folder using default file manager.
    pub fn open_plugins_folder(&self) -> Result<PathBuf> {
        let dir = self.plugins_dir();
        fs::create_dir_all(&dir)?;
        Self::open_folder_native(&dir);
        Ok(dir)
    }

    /// Opens the Wine prefix root folder (or App Config folder on Windows).
    pub fn open_prefix_folder(&self) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            fs::create_dir_all(&self.config_dir)?;
            Self::open_folder_native(&self.config_dir);
            Ok(self.config_dir.clone())
        }
        #[cfg(not(target_os = "windows"))]
        {
            fs::create_dir_all(&self.prefix_dir)?;
            Self::open_folder_native(&self.prefix_dir);
            Ok(self.prefix_dir.clone())
        }
    }

    /// Opens the folder containing the latest Studio installation and ClientSettings.
    pub fn open_studio_folder(&self) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            if let Some(exe) = crate::installer::Installer::find_existing_studio_windows() {
                if let Some(dir) = exe.parent() {
                    Self::open_folder_native(dir);
                    return Ok(dir.to_path_buf());
                }
            }
        }

        let versions_dir = self.versions_dir();
        fs::create_dir_all(&versions_dir)?;

        let mut target_dir = versions_dir.clone();
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            let mut subdirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect();
            subdirs.sort();
            if let Some(last) = subdirs.last() {
                target_dir = last.clone();
            }
        }

        Self::open_folder_native(&target_dir);
        Ok(target_dir)
    }

    /// Helper to open a directory using system file manager (explorer on Windows, xdg-open on Linux).
    pub fn open_folder_native(dir: &Path) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("explorer").arg(dir).spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("xdg-open").arg(dir).spawn();
        }
    }

    /// Copies one or more plugin files into the Roblox Plugins directory.
    pub fn import_plugins(&self, files: &[PathBuf]) -> Result<usize> {
        let target_dir = self.plugins_dir();
        fs::create_dir_all(&target_dir)?;

        let mut count = 0;
        for file in files {
            if let Some(file_name) = file.file_name() {
                let dest = target_dir.join(file_name);
                fs::copy(file, &dest)?;
                info!("Imported plugin to {:?}", dest);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Prepares the Wine prefix and applies Wayland camera fixes & DLL overrides.
    #[allow(dead_code)]
    pub fn setup_prefix(&self, mouse_warp: &str) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let _ = mouse_warp;
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            info!("Ensuring Wine prefix at {:?}", self.prefix_dir);

            // Apply Wine registry tweaks for camera handling (Wayland fix)
            self.apply_mouse_settings(mouse_warp)?;

            // Configure DLL overrides to ensure mock WebView2 is loaded
            self.apply_dll_overrides()?;

            Ok(())
        }
    }

    /// Sets DirectInput & X11 Driver options to eliminate Wayland mouse camera jumping.
    #[allow(dead_code)]
    fn apply_mouse_settings(&self, mouse_warp: &str) -> Result<()> {
        let wine_bin = Self::get_wine_binary();
        debug!("Configuring Wine mouse registry (MouseWarpOverride={})", mouse_warp);
        let _ = Command::new(&wine_bin)
            .env("WINEPREFIX", &self.prefix_dir)
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

        // X11 driver cursor grab confinement
        let _ = Command::new(&wine_bin)
            .env("WINEPREFIX", &self.prefix_dir)
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
            .env("WINEPREFIX", &self.prefix_dir)
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

        Ok(())
    }

    /// Overrides `WebView2Loader` and native runtime DLLs for maximum compatibility.
    #[allow(dead_code)]
    fn apply_dll_overrides(&self) -> Result<()> {
        let wine_bin = Self::get_wine_binary();
        debug!("Configuring Wine DLL overrides");
        for dll in &[
            "WebView2Loader",
            "concrt140",
            "msvcp140",
            "msvcp140_1",
            "msvcp140_2",
            "ucrtbase",
            "vccorlib140",
            "vcruntime140",
            "vcruntime140_1",
        ] {
            let _ = Command::new(&wine_bin)
                .env("WINEPREFIX", &self.prefix_dir)
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

        Ok(())
    }

    /// Generates optimized `dxvk.conf` file based on hardware profile and VRAM limits.
    pub fn write_dxvk_config(
        &self,
        studio_dir: &Path,
        profile: &HardwareProfile,
        max_vram_mb: u32,
        target_fps: u32,
    ) -> Result<()> {
        let threads = profile.recommended_dxvk_compiler_threads();
        let tess_factor = if profile.is_integrated_gpu { 1 } else { 4 };
        let mut config_content = format!(
            "# Generated by rtrust hardware profiler\n\
             dxvk.numCompilerThreads = {}\n\
             dxvk.enableAsync = true\n\
             dxvk.gpl = true\n\
             dxvk.useRawSsbo = true\n\
             d3d11.relaxedBarriers = true\n\
             d3d11.maxTessFactor = {}\n\
             d3d11.syncInterval = 1\n\
             dxgi.syncInterval = 1\n\
             dxvk.tearFree = True\n\
             dxvk.latencySleep = 1\n\
             dxvk.hud = 0\n",
            threads, tess_factor
        );

        if target_fps > 0 {
            config_content.push_str(&format!("dxvk.maxFrameRate = {}\n", target_fps));
        }

        if max_vram_mb > 0 {
            config_content.push_str(&format!(
                "dxgi.maxDeviceMemory = {}\n\
                 dxgi.maxSharedMemory = {}\n",
                max_vram_mb,
                max_vram_mb / 2
            ));
        }

        let target_path = studio_dir.join("dxvk.conf");
        fs::write(&target_path, config_content)?;
        info!("Wrote optimized DXVK configuration to {:?}", target_path);

        Ok(())
    }

    /// Returns the directory where Studio versions are stored.
    pub fn versions_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Some(local_app) = dirs::data_local_dir() {
                let dir = local_app.join("Roblox").join("Versions");
                if dir.exists() {
                    return dir;
                }
            }
        }
        self.data_dir.join("versions")
    }
}
