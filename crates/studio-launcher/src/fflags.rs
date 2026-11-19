use crate::config::LauncherConfig;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tracing::info;

pub struct FFlagsGenerator;

impl FFlagsGenerator {
    /// Generates optimal Fast Flags (FFlags) based on user and hardware configuration.
    pub fn build_settings(cfg: &LauncherConfig) -> Value {
        let shadow_val = if cfg.disable_shadows {
            0
        } else {
            cfg.shadow_intensity
        };

        let mut settings = json!({
            // FPS Target
            "DFIntTaskSchedulerTargetFps": cfg.target_fps,

            // Disable background crash telemetry & analytical bloat
            "FFlagDebugDisableTelemetry": "True",
            "FFlagEnableAnalyticsEvents": "False",

            // 3D Viewport Throttling (Saves massive CPU/GPU when editing scripts or alt-tabbed)
            "FFlagStudioThrottle3DViewWhenInactive": if cfg.throttle_3d_inactive { "True" } else { "False" },
            "FIntStudio3DViewThrottleFPS": 15,
            "FFlagStudioThrottlingEnabled": if cfg.throttle_3d_inactive { "True" } else { "False" },

            // Graphics performance optimizations
            "FFlagDisablePostFx": if cfg.disable_postfx { "True" } else { "False" },
            "FIntRenderShadowIntensity": shadow_val,
            "FFlagFastGPULightBugs": "True",
            "FFlagStudioOptimizeDocking": "True",

            // Editor Graphics Quality Level (1 - 21)
            "DFIntDebugFRMQualityLevelOverride": cfg.editor_quality_level,

            // Camera fix: smooth mouse right-click panning and relative movement
            "FFlagUserCameraToggle": "True",
            "FIntCameraMouseSensitivity": 100,
            "FFlagFixMouseDelta": "True",
            "FFlagFixCameraDelta": "True",
            "FFlagFixFreecamMouseDelta": "True",
            "FFlagStudioCameraRelativeMouseFix": "True",
            "FFlagDisableMouseDeltasWithoutMovement": "True",

            // Bounded concurrency for mesh loading to avoid CPU thrashing
            "FIntMeshContentProviderThreadCount": 4,
            "FIntMeshContentProviderPreloadThreadCount": 2,

            // Texture streaming memory budget
            "FIntTextureStreamingMemoryLimitMB": cfg.texture_memory_mb,
        });

        let obj = settings.as_object_mut().unwrap();

        // Webview2 mitigations
        if cfg.block_webview2 {
            obj.insert("FFlagStudioWebView2Disabled".to_string(), json!("True"));
            obj.insert("FFlagStudioDisableEmbeddedWebViews".to_string(), json!("True"));
            obj.insert("FFlagEnableWebView2".to_string(), json!("False"));
        }

        // Active GPU optimizations:
        // 1. Disable MSAA (reduces 3D viewport pixel fillrate by up to 50% on iGPUs)
        if cfg.disable_msaa {
            obj.insert("FIntDebugForceMSAAQuality".to_string(), json!(0));
            obj.insert("FIntDebugFRMSAAQualityLevel".to_string(), json!(0));
            obj.insert("DFFlagDebugFRMDisableMSAA".to_string(), json!("True"));
        }

        // 2. Force Voxel lighting (bypasses heavy Future Is Bright compute tile passes)
        if cfg.force_voxel_lighting {
            obj.insert("DFFlagDebugRenderForceTechnologyVoxel".to_string(), json!("True"));
            obj.insert("FFlagDebugForcePastLighting".to_string(), json!("True"));
        }

        // 3. Disable procedural terrain grass blade vertex generation
        if cfg.disable_grass {
            obj.insert("FIntFRMMaxGrassDistance".to_string(), json!(0));
            obj.insert("FIntTerrainArraySliceSize".to_string(), json!(4));
        }

        // 4. Viewport target and compute shader culling
        obj.insert("DFIntStudioViewportTargetFps".to_string(), json!(cfg.target_fps));
        obj.insert("FFlagStudioLowFpsMode".to_string(), json!("True"));
        obj.insert("FFlagFastGPULightCulling3".to_string(), json!("True"));
        obj.insert("FFlagFastGPUBoundingBox".to_string(), json!("True"));

        // 5. Heavy post-fx disables
        if cfg.disable_postfx {
            obj.insert("FFlagDebugDisableBloom".to_string(), json!("True"));
            obj.insert("FFlagDebugDisableDepthOfField".to_string(), json!("True"));
            obj.insert("FFlagDebugDisableSunRays".to_string(), json!("True"));
            obj.insert("FFlagDebugDisableColorCorrection".to_string(), json!("True"));
            obj.insert("FFlagDebugDisableSSAO".to_string(), json!("True"));
            obj.insert("FFlagDebugDisableVolumetricClouds".to_string(), json!("True"));
        }

        // Potato profile specific overrides
        if cfg.profile_mode == "potato" || cfg.editor_quality_level <= 2 {
            obj.insert("DFIntTextureQualityOverride".to_string(), json!(1));
        }

        // Graphics backend selection
        match cfg.graphics_backend.as_str() {
            "opengl" => {
                obj.insert("FFlagDebugGraphicsPreferOpenGL".to_string(), json!("True"));
                obj.insert("FFlagDebugGraphicsDisableDirect3D11".to_string(), json!("True"));
            }
            "dx11" | "vulkan" => {
                obj.insert("FFlagDebugGraphicsDisableDirect3D11".to_string(), json!("False"));
            }
            _ => {
                obj.insert("FFlagDebugGraphicsDisableDirect3D11".to_string(), json!("False"));
            }
        }

        // Merge raw custom user FFlags (user flags always take precedence)
        for (k, v) in &cfg.raw_custom_fflags {
            obj.insert(k.clone(), v.clone());
        }

        settings
    }

    /// Writes `ClientAppSettings.json` into the target directory.
    pub fn inject_settings(dir: &Path, cfg: &LauncherConfig) -> std::io::Result<()> {
        let client_settings_dir = dir.join("ClientSettings");
        fs::create_dir_all(&client_settings_dir)?;

        let settings = Self::build_settings(cfg);
        let target_file = client_settings_dir.join("ClientAppSettings.json");

        let formatted = serde_json::to_string_pretty(&settings)?;
        fs::write(&target_file, formatted)?;

        info!("Injected hardware-tailored FFlags to {:?}", target_file);
        Ok(())
    }
}
