use std::fs;
use std::path::Path;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct HardwareProfile {
    pub total_ram_mib: u64,
    pub primary_gpu: GpuVendor,
    pub is_integrated_gpu: bool,
    pub cpu_cores: usize,
}

impl HardwareProfile {
    /// Detects host memory and graphics hardware on Linux.
    pub fn detect() -> Self {
        let total_ram_mib = detect_ram_mib().unwrap_or(8192);
        let (primary_gpu, is_integrated) = detect_gpu();
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        info!(
            "Hardware detected: RAM={} MiB, GPU={:?} (iGPU={}), CPU Cores={}",
            total_ram_mib, primary_gpu, is_integrated, cpu_cores
        );

        Self {
            total_ram_mib,
            primary_gpu,
            is_integrated_gpu: is_integrated,
            cpu_cores,
        }
    }

    /// Calculates ideal compiler threads for DXVK shader compilation.
    /// Preserves spare CPU threads to maintain Wayland compositor smoothness.
    pub fn recommended_dxvk_compiler_threads(&self) -> usize {
        if self.cpu_cores <= 2 {
            1
        } else if self.cpu_cores <= 4 {
            self.cpu_cores.saturating_sub(1)
        } else {
            // Leave at least 2 cores free for display server and Wine audio/IPC
            self.cpu_cores.saturating_sub(2)
        }
    }

    /// Determines if low-memory heuristics should be enabled (e.g. iGPU or <= 8GB RAM).
    #[allow(dead_code)]
    pub fn is_constrained_memory(&self) -> bool {
        self.is_integrated_gpu || self.total_ram_mib <= 8192
    }
}

/// Reads `/proc/meminfo` to determine total system RAM.
fn detect_ram_mib() -> Option<u64> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: u64 = parts[1].parse().ok()?;
                return Some(kb / 1024);
            }
        }
    }
    None
}

/// Inspects `/sys/class/drm` devices to identify the primary graphics vendor and type.
fn detect_gpu() -> (GpuVendor, bool) {
    let drm_path = Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return (GpuVendor::Unknown, false);
    }

    let mut detected_vendor = GpuVendor::Unknown;
    let mut is_integrated = false;

    if let Ok(entries) = fs::read_dir(drm_path) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let name = filename.to_string_lossy();

            // Look for primary card nodes: card0, card1
            if name.starts_with("card") && !name.contains('-') {
                let device_vendor_path = entry.path().join("device/vendor");
                if let Ok(vendor_hex) = fs::read_to_string(device_vendor_path) {
                    let vendor_str = vendor_hex.trim().to_lowercase();
                    match vendor_str.as_str() {
                        "0x8086" => {
                            detected_vendor = GpuVendor::Intel;
                            is_integrated = true;
                            debug!("Found Intel GPU device: {}", name);
                        }
                        "0x1002" => {
                            detected_vendor = GpuVendor::Amd;
                            // Check if APU by inspecting device id or fallback
                            debug!("Found AMD GPU device: {}", name);
                        }
                        "0x10de" => {
                            detected_vendor = GpuVendor::Nvidia;
                            is_integrated = false;
                            debug!("Found NVIDIA GPU device: {}", name);
                            break; // dGPU takes precedence
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    (detected_vendor, is_integrated)
}
