use std::sync::{OnceLock, RwLock};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerfPreset {
    AutoDetect,
    Ultra,
    Balanced,
    BatterySaver,
    Custom,
}

impl PerfPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::AutoDetect => "Auto Detect",
            Self::Ultra => "Ultra",
            Self::Balanced => "Balanced",
            Self::BatterySaver => "Battery Saver",
            Self::Custom => "Custom",
        }
    }

    fn as_key(self) -> &'static str {
        match self {
            Self::AutoDetect => "auto",
            Self::Ultra => "ultra",
            Self::Balanced => "balanced",
            Self::BatterySaver => "battery",
            Self::Custom => "custom",
        }
    }

    fn from_key(s: &str) -> Option<Self> {
        match s.trim() {
            "auto" => Some(Self::AutoDetect),
            "ultra" => Some(Self::Ultra),
            "balanced" => Some(Self::Balanced),
            "battery" => Some(Self::BatterySaver),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HardwareProfile {
    pub total_ram_mb: u64,
    pub cpu_cores: usize,
    pub gpu_name: String,
    pub gpu_backend: String,
    pub gpu_device_type: String,
    pub is_software_gpu: bool,
}

#[derive(Clone, Debug)]
pub struct TunerConfig {
    pub preset: PerfPreset,
    pub cache_budget_mb: u32,
    pub luau_gc_mb: u32,
    pub render_scale: f32,
    pub fps_limit: u32, // 0 = Unlimited, 30, 60
    pub shadows_enabled: bool,
}

struct TunerState {
    hardware: HardwareProfile,
    config: TunerConfig,
}

static STATE: OnceLock<RwLock<TunerState>> = OnceLock::new();

fn detect_total_ram_mb() -> u64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    let kb: u64 = rest
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if kb > 0 {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    4096
}

fn detect_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn config_for_ram(ram_mb: u64, preset: PerfPreset) -> TunerConfig {
    match preset {
        PerfPreset::Ultra => TunerConfig {
            preset,
            cache_budget_mb: 2048,
            luau_gc_mb: 256,
            render_scale: 0.75,
            fps_limit: 60,
            shadows_enabled: false,
        },
        PerfPreset::Balanced => TunerConfig {
            preset,
            cache_budget_mb: 1024,
            luau_gc_mb: 256,
            render_scale: 0.50,
            fps_limit: 60,
            shadows_enabled: false,
        },
        PerfPreset::BatterySaver => TunerConfig {
            preset,
            cache_budget_mb: 1024,
            luau_gc_mb: 128,
            render_scale: 0.35,
            fps_limit: 60,
            shadows_enabled: false,
        },
        PerfPreset::AutoDetect => {
            if ram_mb >= 7600 {
                TunerConfig {
                    preset,
                    cache_budget_mb: 3072,
                    luau_gc_mb: 256,
                    render_scale: 1.0,
                    fps_limit: 60,
                    shadows_enabled: true,
                }
            } else if ram_mb >= 3800 {
                TunerConfig {
                    preset,
                    cache_budget_mb: 1536,
                    luau_gc_mb: 256,
                    render_scale: 0.50,
                    fps_limit: 60,
                    shadows_enabled: false,
                }
            } else {
                TunerConfig {
                    preset,
                    cache_budget_mb: 1024,
                    luau_gc_mb: 128,
                    render_scale: 0.40,
                    fps_limit: 60,
                    shadows_enabled: false,
                }
            }
        }
        PerfPreset::Custom => TunerConfig {
            preset,
            cache_budget_mb: 1024,
            luau_gc_mb: 256,
            render_scale: 0.50,
            fps_limit: 60,
            shadows_enabled: false,
        },
    }
}

fn config_file_path() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("RUSTUDIO_DATA_DIR") {
        return std::path::PathBuf::from(dir).join("hardware_tuner.cfg");
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(xdg).join("rustudio").join("hardware_tuner.cfg");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home).join(".config").join("rustudio").join("hardware_tuner.cfg");
    }
    std::env::temp_dir().join("rustudio").join("hardware_tuner.cfg")
}

fn save_config(cfg: &TunerConfig) {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = format!(
        "preset={}\ncache_budget_mb={}\nluau_gc_mb={}\nrender_scale={:.3}\nfps_limit={}\nshadows_enabled={}\n",
        cfg.preset.as_key(),
        cfg.cache_budget_mb,
        cfg.luau_gc_mb,
        cfg.render_scale,
        cfg.fps_limit,
        cfg.shadows_enabled
    );
    let _ = std::fs::write(path, body);
}

fn load_config(ram_mb: u64) -> Option<TunerConfig> {
    let text = std::fs::read_to_string(config_file_path()).ok()?;
    let mut preset = PerfPreset::AutoDetect;
    let mut cfg = config_for_ram(ram_mb, PerfPreset::Custom);
    let mut has_any = false;
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "preset" => {
                if let Some(p) = PerfPreset::from_key(v) {
                    preset = p;
                    has_any = true;
                }
            }
            "cache_budget_mb" => {
                if let Ok(n) = v.trim().parse::<u32>() {
                    cfg.cache_budget_mb = n.clamp(512, 3072);
                }
            }
            "luau_gc_mb" => {
                if let Ok(n) = v.trim().parse::<u32>() {
                    cfg.luau_gc_mb = n.clamp(16, 512);
                }
            }
            "render_scale" => {
                if let Ok(n) = v.trim().parse::<f32>() {
                    cfg.render_scale = n.clamp(0.05, 1.25);
                }
            }
            "fps_limit" => {
                if let Ok(n) = v.trim().parse::<u32>() {
                    cfg.fps_limit = n;
                }
            }
            "shadows_enabled" => {
                if let Ok(b) = v.trim().parse::<bool>() {
                    cfg.shadows_enabled = b;
                }
            }
            _ => {}
        }
    }
    if !has_any {
        return None;
    }
    cfg.preset = preset;
    if preset != PerfPreset::Custom {
        return Some(config_for_ram(ram_mb, preset));
    }
    Some(cfg)
}

fn apply_env_hints(cfg: &TunerConfig, hw: &HardwareProfile) {
    let lowend = cfg.preset == PerfPreset::BatterySaver
        || cfg.render_scale <= 0.75
        || hw.is_software_gpu
        || hw.total_ram_mb < 3800;
    let few_cores = hw.cpu_cores <= 2 || cfg.preset == PerfPreset::BatterySaver;
    unsafe {
        if lowend {
            std::env::set_var("RUSTUDIO_LOWEND", "1");
        } else if cfg.preset == PerfPreset::Ultra {
            std::env::set_var("RUSTUDIO_LOWEND", "0");
        }
        if few_cores {
            std::env::set_var("RUSTUDIO_PLAY_WORKER", "0");
            if std::env::var_os("RAYON_NUM_THREADS").is_none() {
                std::env::set_var("RAYON_NUM_THREADS", "2");
            }
        } else {
            std::env::set_var("RUSTUDIO_PLAY_WORKER", "1");
        }
    }
}

pub fn apply_startup_hints() {
    let _ = state();
}

fn state() -> &'static RwLock<TunerState> {
    STATE.get_or_init(|| {
        let total_ram_mb = detect_total_ram_mb();
        let cpu_cores = detect_cpu_cores();
        let hardware = HardwareProfile {
            total_ram_mb,
            cpu_cores,
            gpu_name: "Detecting GPU...".to_owned(),
            gpu_backend: "Auto".to_owned(),
            gpu_device_type: "Unknown".to_owned(),
            is_software_gpu: false,
        };
        let config = load_config(total_ram_mb)
            .unwrap_or_else(|| config_for_ram(total_ram_mb, PerfPreset::AutoDetect));
        crate::mobile::set_eco(config.render_scale < 0.8);
        apply_env_hints(&config, &hardware);
        RwLock::new(TunerState { hardware, config })
    })
}

pub fn register_gpu(name: &str, backend: &str, device_type: &str) {
    let lower = name.to_ascii_lowercase();
    let is_sw = lower.contains("llvmpipe")
        || lower.contains("swiftshader")
        || lower.contains("lavapipe")
        || device_type.eq_ignore_ascii_case("Cpu");
    if let Ok(mut guard) = state().write() {
        guard.hardware.gpu_name = name.to_owned();
        guard.hardware.gpu_backend = backend.to_owned();
        guard.hardware.gpu_device_type = device_type.to_owned();
        guard.hardware.is_software_gpu = is_sw;
        if is_sw && guard.config.preset == PerfPreset::AutoDetect {
            // Avoid melting CPU on software rasterizers
            guard.config.render_scale = guard.config.render_scale.min(0.75);
            guard.config.shadows_enabled = false;
        }
        let cfg = guard.config.clone();
        let hw = guard.hardware.clone();
        apply_env_hints(&cfg, &hw);
    }
}

pub fn hardware_profile() -> HardwareProfile {
    state()
        .read()
        .map(|g| g.hardware.clone())
        .unwrap_or_else(|_| HardwareProfile {
            total_ram_mb: 4096,
            cpu_cores: 4,
            gpu_name: "Unknown".to_owned(),
            gpu_backend: "Unknown".to_owned(),
            gpu_device_type: "Unknown".to_owned(),
            is_software_gpu: false,
        })
}

pub fn config() -> TunerConfig {
    state()
        .read()
        .map(|g| g.config.clone())
        .unwrap_or_else(|_| config_for_ram(4096, PerfPreset::AutoDetect))
}

pub fn apply_preset(preset: PerfPreset) {
    if let Ok(mut guard) = state().write() {
        let ram = guard.hardware.total_ram_mb;
        let mut cfg = if preset == PerfPreset::Custom {
            let mut c = guard.config.clone();
            c.preset = PerfPreset::Custom;
            c
        } else {
            config_for_ram(ram, preset)
        };
        if guard.hardware.is_software_gpu && preset == PerfPreset::AutoDetect {
            cfg.render_scale = cfg.render_scale.min(0.75);
            cfg.shadows_enabled = false;
        }
        crate::mobile::set_eco(cfg.render_scale < 0.8);
        let hw = guard.hardware.clone();
        apply_env_hints(&cfg, &hw);
        save_config(&cfg);
        guard.config = cfg;
    }
}

pub fn update_config(mut f: impl FnMut(&mut TunerConfig)) {
    if let Ok(mut guard) = state().write() {
        f(&mut guard.config);
        guard.config.preset = PerfPreset::Custom;
        guard.config.cache_budget_mb = guard.config.cache_budget_mb.clamp(512, 3072);
        guard.config.luau_gc_mb = guard.config.luau_gc_mb.clamp(16, 512);
        guard.config.render_scale = guard.config.render_scale.clamp(0.05, 1.25);
        crate::mobile::set_eco(guard.config.render_scale < 0.8);
        let cfg = guard.config.clone();
        let hw = guard.hardware.clone();
        apply_env_hints(&cfg, &hw);
        save_config(&cfg);
    }
}

pub fn render_scale() -> f32 {
    state()
        .read()
        .map(|g| g.config.render_scale.clamp(0.5, 1.0))
        .unwrap_or(1.0)
}

pub fn shadows_enabled() -> bool {
    state()
        .read()
        .map(|g| g.config.shadows_enabled)
        .unwrap_or(true)
}

pub fn fps_limit() -> u32 {
    state()
        .read()
        .map(|g| g.config.fps_limit)
        .unwrap_or(60)
}

pub fn luau_gc_mb() -> u32 {
    state()
        .read()
        .map(|g| g.config.luau_gc_mb)
        .unwrap_or(64)
}

pub fn cache_budget_mb() -> u32 {
    state()
        .read()
        .map(|g| g.config.cache_budget_mb)
        .unwrap_or(256)
}

pub fn play_frame_interval() -> Option<Duration> {
    let fps = fps_limit();
    if fps == 0 {
        None
    } else {
        Some(Duration::from_secs_f64(1.0 / fps as f64))
    }
}

pub fn current_rss_mb() -> f32 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    let kb: f32 = rest
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);
                    return kb / 1024.0;
                }
            }
        }
    }
    0.0
}
