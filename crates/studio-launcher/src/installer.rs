use crate::api::{PackageItem, RobloxApi};
use crate::config::LauncherConfig;
use crate::fflags::FFlagsGenerator;
use crate::hardware::HardwareProfile;
use crate::prefix::PrefixManager;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info};

pub type InstallProgressCallback = Arc<dyn Fn(u64, u64, &str) + Send + Sync>;

pub const STUB_WEBVIEW2_BYTES: &[u8] = include_bytes!("../../../assets/dll/WebView2Loader.dll");

pub struct Installer {
    api: RobloxApi,
    prefix_mgr: Arc<PrefixManager>,
}

impl Installer {
    pub fn new(prefix_mgr: Arc<PrefixManager>) -> Self {
        Self {
            api: RobloxApi::new(),
            prefix_mgr,
        }
    }

    /// Performs Safe DLL Swap for Roblox Studio:
    /// - If `enable` is true: backs up original `WebView2Loader.dll` -> `WebView2Loader.dll.bak`
    ///   and replaces it with the embedded ultra-light stub DLL (`STUB_WEBVIEW2_BYTES`).
    /// - If `enable` is false: restores `WebView2Loader.dll.bak` -> `WebView2Loader.dll`.
    pub fn safe_swap_webview2(studio_dir: &Path, enable: bool) -> Result<()> {
        let dll_path = studio_dir.join("WebView2Loader.dll");
        let bak_path = studio_dir.join("WebView2Loader.dll.bak");

        if enable {
            if !bak_path.exists() && dll_path.exists() {
                if let Err(e) = fs::copy(&dll_path, &bak_path) {
                    tracing::warn!("Could not backup original WebView2Loader.dll: {}", e);
                } else {
                    info!("Backed up original WebView2Loader.dll -> WebView2Loader.dll.bak");
                }
            }
            fs::write(&dll_path, STUB_WEBVIEW2_BYTES)
                .context("Failed to write stub WebView2Loader.dll")?;
            info!("Injected lightweight stub WebView2Loader.dll into {:?}", dll_path);
        } else if bak_path.exists() {
            fs::copy(&bak_path, &dll_path)
                .context("Failed to restore original WebView2Loader.dll from backup")?;
            info!("Restored original WebView2Loader.dll from backup in {:?}", studio_dir);
        }
        Ok(())
    }

    /// Searches standard Windows installation directories for an existing Roblox Studio installation.
    #[allow(dead_code)]
    pub fn find_existing_studio_windows() -> Option<PathBuf> {
        let mut search_dirs = Vec::new();

        // 1. %LOCALAPPDATA%\Roblox\Versions
        if let Some(local_app) = dirs::data_local_dir() {
            search_dirs.push(local_app.join("Roblox").join("Versions"));
        }

        // 2. Program Files paths
        if let Ok(prog_files) = std::env::var("ProgramFiles(x86)") {
            search_dirs.push(PathBuf::from(prog_files).join("Roblox").join("Versions"));
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            search_dirs.push(PathBuf::from(prog_files).join("Roblox").join("Versions"));
        }

        for dir in search_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&dir) {
                let mut candidates = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let exe = path.join("RobloxStudioBeta.exe");
                        if exe.exists() {
                            let mtime = exe
                                .metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                            candidates.push((mtime, exe));
                        }
                    }
                }
                candidates.sort_by(|a, b| b.0.cmp(&a.0));
                if let Some((_, exe)) = candidates.into_iter().next() {
                    info!("Detected existing Windows Roblox Studio: {:?}", exe);
                    return Some(exe);
                }
            }
        }

        None
    }

    /// Ensures the latest version of Roblox Studio is downloaded and ready.
    pub async fn ensure_latest_studio(
        &self,
        profile: &HardwareProfile,
        cfg: &LauncherConfig,
    ) -> Result<PathBuf> {
        self.ensure_latest_studio_with_progress(profile, cfg, None).await
    }

    /// Ensures the latest version of Roblox Studio is downloaded and ready with progress reporting.
    pub async fn ensure_latest_studio_with_progress(
        &self,
        profile: &HardwareProfile,
        cfg: &LauncherConfig,
        progress_cb: Option<InstallProgressCallback>,
    ) -> Result<PathBuf> {
        let version_info = self.api.fetch_latest_version().await?;
        let version_upload = &version_info.client_version_upload;

        let version_dir = self.prefix_mgr.versions_dir().join(version_upload);
        let executable_path = version_dir.join("RobloxStudioBeta.exe");
        let content_fonts = version_dir.join("content/fonts");

        if executable_path.exists() && content_fonts.exists() {
            info!("Roblox Studio version {} is up to date.", version_upload);
        } else {
            info!("Installing Roblox Studio version {}...", version_upload);
            self.download_and_extract_version(version_upload, &version_dir, progress_cb)
                .await?;
        }

        // Apply hardware profile FFlags and DXVK configuration
        FFlagsGenerator::inject_settings(&version_dir, cfg)?;
        self.prefix_mgr
            .write_dxvk_config(&version_dir, profile, cfg.dxvk_max_vram_mb, cfg.target_fps)?;

        // Deploy or toggle Rust DXGI Proxy Shim (dxgi.dll) for active GPU throttling
        let _ = crate::runtime::SlimRuntimeManager::deploy_dxgi_shim(
            &version_dir,
            cfg.use_dxgi_shim,
            cfg.alt_tab_gpu_sleep,
            cfg.vram_cap_mb,
        );

        // Ensure Qt5 platform plugins and qt.conf are configured
        Self::configure_qt_environment(&version_dir)?;

        // Ensure AppSettings.xml is written (critical for content resolution)
        Self::write_app_settings(&version_dir)?;

        // Swap or restore WebView2Loader based on active profile & user configuration
        let potato_mode = cfg.profile_mode == "potato" && cfg.block_webview2;
        Self::safe_swap_webview2(&version_dir, potato_mode)?;

        Ok(executable_path)
    }

    /// Writes standard AppSettings.xml required by Roblox Studio.
    fn write_app_settings(studio_dir: &Path) -> Result<()> {
        let app_settings_path = studio_dir.join("AppSettings.xml");
        let content = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
                       <Settings>\r\n\
                               <ContentFolder>content</ContentFolder>\r\n\
                               <BaseUrl>http://www.roblox.com</BaseUrl>\r\n\
                       </Settings>\r\n";
        fs::write(&app_settings_path, content)?;
        info!("Wrote AppSettings.xml to {:?}", app_settings_path);
        Ok(())
    }

    /// Configures qt.conf and symlinks so Qt5 windows platform plugins load cleanly under Wine.
    fn configure_qt_environment(studio_dir: &Path) -> Result<()> {
        let qt_conf = studio_dir.join("qt.conf");
        let content = "[Paths]\nPrefix = .\nPlugins = Qt5\n";
        let _ = fs::write(qt_conf, content);

        let qt5_dir = studio_dir.join("Qt5");
        if qt5_dir.exists() {
            let links = ["platforms", "imageformats", "styles"];
            for sub in links {
                let target = studio_dir.join(sub);
                let source = qt5_dir.join(sub);
                if source.exists() && !target.exists() {
                    #[cfg(unix)]
                    let _ = std::os::unix::fs::symlink(&source, &target);
                }
            }
        }
        Ok(())
    }

    /// Downloads and extracts all Studio packages concurrently.
    async fn download_and_extract_version(
        &self,
        version_upload: &str,
        dest_dir: &Path,
        progress_cb: Option<InstallProgressCallback>,
    ) -> Result<()> {
        fs::create_dir_all(dest_dir)?;
        let manifest = self.api.fetch_package_manifest(version_upload).await?;

        let total_packages = manifest.len() as u64;
        info!("Manifest contains {} archive packages", total_packages);

        let pb = Arc::new(ProgressBar::new(total_packages));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        pb.set_message("Starting package downloads...");

        let semaphore = Arc::new(Semaphore::new(6)); // Concurrency limit
        let completed_count = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let tasks: Vec<_> = manifest
            .into_iter()
            .map(|pkg| {
                let sem = semaphore.clone();
                let upload = version_upload.to_string();
                let dest = dest_dir.to_path_buf();
                let progress = pb.clone();
                let completed = completed_count.clone();
                let cb = progress_cb.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    progress.set_message(format!("Downloading {}", pkg.filename));
                    let res = download_and_unpack_package(&upload, &pkg, &dest).await;
                    progress.inc(1);
                    let current = completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if let Some(ref callback) = cb {
                        callback(current, total_packages, &pkg.filename);
                    }
                    progress.set_message(format!("Extracted {}", pkg.filename));
                    res
                })
            })
            .collect();

        for task in tasks {
            task.await??;
        }

        pb.finish_with_message("All packages downloaded and extracted successfully!");
        info!("All packages extracted to {:?}", dest_dir);
        Ok(())
    }

    /// Installs the embedded mock WebView2Loader.dll stub into the Studio directory.
    fn install_mock_webview2(&self, studio_dir: &Path) -> Result<()> {
        Self::safe_swap_webview2(studio_dir, true)
    }
}

/// Maps a Roblox package archive filename to its designated installation subdirectory.
/// Extracted directly from official Roblox installer deployment specifications.
pub fn package_destination(pkg_name: &str) -> PathBuf {
    match pkg_name {
        "ApplicationConfig.zip" => PathBuf::from("ApplicationConfig"),
        "content-avatar.zip" => PathBuf::from("content/avatar"),
        "content-configs.zip" => PathBuf::from("content/configs"),
        "content-fonts.zip" => PathBuf::from("content/fonts"),
        "content-sky.zip" => PathBuf::from("content/sky"),
        "content-sounds.zip" => PathBuf::from("content/sounds"),
        "content-textures2.zip" => PathBuf::from("content/textures"),
        "content-studio_svg_textures.zip" => PathBuf::from("content/studio_svg_textures"),
        "content-models.zip" => PathBuf::from("content/models"),
        "content-textures3.zip" => PathBuf::from("PlatformContent/pc/textures"),
        "content-terrain.zip" => PathBuf::from("PlatformContent/pc/terrain"),
        "content-platform-fonts.zip" => PathBuf::from("PlatformContent/pc/fonts"),
        "content-platform-dictionaries.zip" => {
            PathBuf::from("PlatformContent/pc/shared_compression_dictionaries")
        }
        "content-qt_translations.zip" => PathBuf::from("content/qt_translations"),
        "content-api-docs.zip" => PathBuf::from("content/api_docs"),
        "extracontent-scripts.zip" => PathBuf::from("ExtraContent/scripts"),
        "extracontent-luapackages.zip" => PathBuf::from("ExtraContent/LuaPackages"),
        "extracontent-translations.zip" => PathBuf::from("ExtraContent/translations"),
        "extracontent-models.zip" => PathBuf::from("ExtraContent/models"),
        "extracontent-textures.zip" => PathBuf::from("ExtraContent/textures"),
        "studiocontent-models.zip" => PathBuf::from("StudioContent/models"),
        "studiocontent-textures.zip" => PathBuf::from("StudioContent/textures"),
        "shaders.zip" => PathBuf::from("shaders"),
        "BuiltInPlugins.zip" => PathBuf::from("BuiltInPlugins"),
        "BuiltInStandalonePlugins.zip" => PathBuf::from("BuiltInStandalonePlugins"),
        "Plugins.zip" => PathBuf::from("Plugins"),
        "StudioFonts.zip" => PathBuf::from("StudioFonts"),
        "ssl.zip" => PathBuf::from("ssl"),
        "WebView2RuntimeInstaller.zip" => PathBuf::from("WebView2RuntimeInstaller"),
        _ => PathBuf::new(),
    }
}

async fn download_and_unpack_package(
    version_upload: &str,
    pkg: &PackageItem,
    dest_dir: &Path,
) -> Result<()> {
    let url = RobloxApi::package_download_url(version_upload, &pkg.filename);
    debug!("Downloading {}", pkg.filename);

    let sub_dest = package_destination(&pkg.filename);
    let target_dir = if sub_dest.as_os_str().is_empty() {
        dest_dir.to_path_buf()
    } else {
        dest_dir.join(sub_dest)
    };
    fs::create_dir_all(&target_dir)?;

    let bytes = reqwest::get(&url)
        .await?
        .bytes()
        .await
        .with_context(|| format!("Failed to read stream for {}", pkg.filename))?;

    // Most Studio packages are zip archives
    if pkg.filename.ends_with(".zip") {
        let reader = Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(reader)?;

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;

            // Roblox packages contain Windows backslashes in zip entry paths.
            // Normalize them to POSIX directory separators.
            let normalized = file.name().replace('\\', "/");
            let clean_rel_path = PathBuf::from(normalized);

            let mut safe_path = PathBuf::new();
            for comp in clean_rel_path.components() {
                if let std::path::Component::Normal(c) = comp {
                    safe_path.push(c);
                }
            }

            if safe_path.as_os_str().is_empty() {
                continue;
            }

            let outpath = target_dir.join(&safe_path);

            if file.is_dir() || file.name().ends_with('/') || file.name().ends_with('\\') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
    } else {
        // Plain file
        let outpath = target_dir.join(&pkg.filename);
        fs::write(outpath, bytes)?;
    }

    Ok(())
}
