use crate::hardware::HardwareProfile;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlace {
    pub name: String,
    pub place_id: u64,
    pub description: String,
    #[serde(default)]
    pub universe_id: Option<u64>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub team_create_enabled: Option<bool>,
    #[serde(default = "default_personal")]
    pub category: String, // "personal" | "group" | "shared" | "local"
    #[serde(default)]
    pub local_path: Option<String>,
}

fn default_personal() -> String {
    "personal".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub username: String,
    pub display_name: String,
    pub user_id: u64,
    pub roblosecurity: String,
    pub avatar_url: Option<String>,
}

impl UserProfile {
    pub fn session_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rtrust/session.json")
    }

    pub fn load_session() -> Option<Self> {
        let path = Self::session_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(profile) = serde_json::from_str::<UserProfile>(&data) {
                return Some(profile);
            }
        }
        None
    }

    pub fn save_session(&self) -> std::io::Result<()> {
        let path = Self::session_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn delete_session() -> std::io::Result<()> {
        let path = Self::session_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn default_fps() -> u32 {
    60
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_texture_memory() -> u32 {
    512
}
fn default_dxvk_threads() -> usize {
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    if cpu_cores > 2 {
        cpu_cores - 2
    } else {
        1
    }
}
fn default_mouse_warp() -> String {
    "disable".to_string()
}
fn default_resolution() -> String {
    "1920x1080".to_string()
}
fn default_profile_mode() -> String {
    "auto".to_string()
}
fn default_graphics_backend() -> String {
    "auto".to_string()
}
fn default_quality_level() -> u32 {
    16
}
fn default_wallpaper_mode() -> String {
    "auto".to_string()
}
fn default_wallpaper_preset() -> String {
    "studio-dark".to_string()
}
fn default_wallpaper_blur() -> u32 {
    0
}
fn default_wallpaper_dim() -> u32 {
    45
}
pub fn default_saved_places() -> Vec<SavedPlace> {
    vec![
        SavedPlace {
            name: "Default Baseplate".to_string(),
            place_id: 95206881,
            description: "Standard Roblox Studio Template".to_string(),
            universe_id: Some(28220420),
            thumbnail_url: Some("https://tr.rbxcdn.com/180DAY-2f86416cf82c109ff80b017014bec852/512/512/Image/Png/noFilter".to_string()),
            team_create_enabled: Some(false),
            category: "personal".to_string(),
            local_path: None,
        },
        SavedPlace {
            name: "Testing Sandbox".to_string(),
            place_id: 1818,
            description: "Quick Prototype & Script Testing".to_string(),
            universe_id: None,
            thumbnail_url: None,
            team_create_enabled: Some(false),
            category: "personal".to_string(),
            local_path: None,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "default_fps")]
    pub target_fps: u32,
    pub place_id: Option<u64>,
    #[serde(default = "default_true")]
    pub throttle_3d_inactive: bool,
    #[serde(default = "default_texture_memory")]
    pub texture_memory_mb: u32,
    #[serde(default)]
    pub shadow_intensity: u32,
    #[serde(default = "default_true")]
    pub disable_postfx: bool,
    #[serde(default = "default_dxvk_threads")]
    pub dxvk_threads: usize,
    #[serde(default = "default_true")]
    pub force_xwayland_mouse: bool,
    #[serde(default = "default_mouse_warp")]
    pub mouse_warp_mode: String,
    #[serde(default = "default_false")]
    pub virtual_desktop: bool,
    #[serde(default = "default_resolution")]
    pub virtual_desktop_res: String,
    #[serde(default = "default_true")]
    pub auto_hide_launcher: bool,
    pub user_profile: Option<UserProfile>,
    #[serde(default = "default_saved_places")]
    pub saved_places: Vec<SavedPlace>,

    // New profile & optimization fields
    #[serde(default = "default_profile_mode")]
    pub profile_mode: String, // "auto", "potato", "balanced", "high", "custom"
    #[serde(default = "default_graphics_backend")]
    pub graphics_backend: String, // "auto", "vulkan", "dx11", "opengl"
    #[serde(default)]
    pub dxvk_max_vram_mb: u32, // 0 = unlimited, or 1024, 2048, etc.
    #[serde(default = "default_quality_level")]
    pub editor_quality_level: u32, // 1 - 21 (default 16)
    #[serde(default = "default_false")]
    pub disable_shadows: bool,
    #[serde(default = "default_true")]
    pub block_webview2: bool,
    pub custom_runner_path: Option<String>,
    pub custom_env_vars: Option<String>,
    pub custom_cli_flags: Option<String>,
    #[serde(default)]
    pub raw_custom_fflags: serde_json::Map<String, serde_json::Value>,

    // Wallpaper customization fields
    #[serde(default = "default_wallpaper_mode")]
    pub wallpaper_mode: String, // "auto", "local", "preset"
    #[serde(default = "default_wallpaper_preset")]
    pub wallpaper_preset: String, // "studio-dark", "neon-grid", "synthwave", "cyber-void"
    pub wallpaper_path: Option<String>,
    #[serde(default = "default_wallpaper_blur")]
    pub wallpaper_blur: u32, // 0 - 30 (default 12)
    #[serde(default = "default_wallpaper_dim")]
    pub wallpaper_dim: u32, // 0 - 90 (default 45)

    // Onboarding
    #[serde(default = "default_false")]
    pub first_run_completed: bool,

    // Rust DXGI Shim & GPU Resource Controls
    #[serde(default = "default_true")]
    pub use_dxgi_shim: bool,
    #[serde(default = "default_true")]
    pub alt_tab_gpu_sleep: bool,
    #[serde(default = "default_vram_cap")]
    pub vram_cap_mb: u32,
    #[serde(default = "default_cgroup_ram")]
    pub cgroup_ram_limit_gb: u32,
    #[serde(default = "default_true")]
    pub disable_msaa: bool,
    #[serde(default = "default_false")]
    pub force_voxel_lighting: bool,
    #[serde(default = "default_true")]
    pub disable_grass: bool,
}

fn default_vram_cap() -> u32 {
    1024
}
fn default_cgroup_ram() -> u32 {
    6
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            target_fps: default_fps(),
            place_id: None,
            throttle_3d_inactive: default_true(),
            texture_memory_mb: default_texture_memory(),
            shadow_intensity: 0,
            disable_postfx: default_true(),
            dxvk_threads: default_dxvk_threads(),
            force_xwayland_mouse: default_true(),
            mouse_warp_mode: default_mouse_warp(),
            virtual_desktop: default_false(),
            virtual_desktop_res: default_resolution(),
            auto_hide_launcher: default_true(),
            user_profile: None,
            saved_places: default_saved_places(),
            profile_mode: default_profile_mode(),
            graphics_backend: default_graphics_backend(),
            dxvk_max_vram_mb: 0,
            editor_quality_level: default_quality_level(),
            disable_shadows: default_false(),
            block_webview2: default_false(),
            custom_runner_path: None,
            custom_env_vars: None,
            custom_cli_flags: None,
            raw_custom_fflags: serde_json::Map::new(),
            wallpaper_mode: default_wallpaper_mode(),
            wallpaper_preset: default_wallpaper_preset(),
            wallpaper_path: None,
            wallpaper_blur: default_wallpaper_blur(),
            wallpaper_dim: default_wallpaper_dim(),
            first_run_completed: default_false(),
            use_dxgi_shim: default_true(),
            alt_tab_gpu_sleep: default_true(),
            vram_cap_mb: default_vram_cap(),
            cgroup_ram_limit_gb: default_cgroup_ram(),
            disable_msaa: default_true(),
            force_voxel_lighting: default_false(),
            disable_grass: default_true(),
        }
    }
}

impl LauncherConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rtrust/launcher_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn apply_profile(&mut self, profile: &str, hw: &HardwareProfile) {
        match profile {
            "potato" => {
                self.profile_mode = "potato".to_string();
                self.target_fps = 60;
                self.throttle_3d_inactive = true;
                self.texture_memory_mb = 256;
                self.shadow_intensity = 0;
                self.disable_shadows = true;
                self.disable_postfx = true;
                self.editor_quality_level = 1;
                self.dxvk_max_vram_mb = 1024;
                self.block_webview2 = true;
                self.graphics_backend = "vulkan".to_string();
                self.use_dxgi_shim = true;
                self.alt_tab_gpu_sleep = true;
                self.vram_cap_mb = 1024;
                self.cgroup_ram_limit_gb = 4;
                self.disable_msaa = true;
                self.force_voxel_lighting = true;
                self.disable_grass = true;
            }
            "balanced" => {
                self.profile_mode = "balanced".to_string();
                self.target_fps = 60;
                self.throttle_3d_inactive = true;
                self.texture_memory_mb = 1024;
                self.shadow_intensity = 1;
                self.disable_shadows = false;
                self.disable_postfx = false;
                self.editor_quality_level = 10;
                self.dxvk_max_vram_mb = 2048;
                self.block_webview2 = false;
                self.graphics_backend = "vulkan".to_string();
                self.use_dxgi_shim = true;
                self.alt_tab_gpu_sleep = true;
                self.vram_cap_mb = 2048;
                self.cgroup_ram_limit_gb = 6;
                self.disable_msaa = true;
                self.force_voxel_lighting = false;
                self.disable_grass = true;
            }
            "high" => {
                self.profile_mode = "high".to_string();
                self.target_fps = 144;
                self.throttle_3d_inactive = false;
                self.texture_memory_mb = 2048;
                self.shadow_intensity = 2;
                self.disable_shadows = false;
                self.disable_postfx = false;
                self.editor_quality_level = 21;
                self.dxvk_max_vram_mb = 0; // Unlimited
                self.block_webview2 = false;
                self.graphics_backend = "vulkan".to_string();
                self.use_dxgi_shim = false;
                self.alt_tab_gpu_sleep = false;
                self.vram_cap_mb = 0;
                self.cgroup_ram_limit_gb = 0;
                self.disable_msaa = false;
                self.force_voxel_lighting = false;
                self.disable_grass = false;
            }
            "auto" => {
                if hw.is_constrained_memory() {
                    self.apply_profile("potato", hw);
                } else if hw.total_ram_mib <= 16384 {
                    self.apply_profile("balanced", hw);
                } else {
                    self.apply_profile("high", hw);
                }
                self.profile_mode = "auto".to_string();
            }
            _ => {
                self.profile_mode = "custom".to_string();
            }
        }
    }
}
