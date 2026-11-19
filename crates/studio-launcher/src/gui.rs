use crate::api::{RobloxApi, TrendingExperience};
use crate::config::{LauncherConfig, SavedPlace, UserProfile};
use crate::fflags::FFlagsGenerator;
use crate::hardware::HardwareProfile;
use crate::installer::Installer;
use crate::ipc::IpcServer;
use crate::prefix::PrefixManager;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};
use tracing::{error, info};
use wry::WebViewBuilder;

#[cfg(target_os = "linux")]
use gtk::prelude::*;
#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingIpc {
    DragWindow,
    Minimize,
    Maximize,
    Close,
    StartWebviewLogin,
    StartQuickLogin,
    OpenPluginsFolder,
    ImportPlugins,
    OpenStudioFolder,
    OpenPrefixFolder,
    SignOut,
    OpenUrl {
        url: String,
    },
    AddPlace {
        name: String,
        place_id: u64,
    },
    SaveConfig {
        config: serde_json::Value,
    },
    Play {
        place_id: Option<u64>,
        fps: Option<u32>,
        force_xwayland: Option<bool>,
    },
    Init,
    // New optimization & profile controls
    ApplyProfile {
        profile: String,
    },
    ResetRecommendedProfile,
    SaveRawFflags {
        fflags: serde_json::Map<String, serde_json::Value>,
    },
    PickWallpaperFile,
    CompleteOnboarding {
        profile: String,
    },
    RestoreOriginalDll,
    CheckUpdate,
    PerformStudioUpdate,
    DownloadPlaceSnapshot {
        place_id: u64,
    },
    RefreshExperiences,
    PickLocalPlaceFile,
    FetchSubplaces {
        universe_id: u64,
    },
}

#[derive(Debug)]
pub enum UserEvent {
    DragWindow,
    Minimize,
    Maximize,
    Close,
    StartWebviewLogin,
    StartQuickLogin,
    CheckWebviewCookies,
    OpenPluginsFolder,
    ImportPlugins,
    OpenStudioFolder,
    OpenPrefixFolder,
    ShowToast {
        message: String,
        is_success: bool,
    },
    QuickLoginReady {
        code: String,
        qr_url: Option<String>,
    },
    QuickLoginError(String),
    LoginSuccess(UserProfile),
    Play {
        place_id: Option<u64>,
        fps: u32,
        force_xwayland: bool,
    },
    SignOut,
    OpenUrl {
        url: String,
    },
    AddPlace {
        name: String,
        place_id: u64,
    },
    SaveConfig {
        config: serde_json::Value,
    },
    #[allow(dead_code)]
    TrendingLoaded(Vec<TrendingExperience>),
    InstallProgress {
        current: u64,
        total: u64,
        pkg: String,
    },
    SendStateToJs,
    StudioStarted,
    StudioExited(i32),
    StatusUpdate(String),
    // New user events
    ApplyProfile(String),
    ResetRecommendedProfile,
    SaveRawFflags(serde_json::Map<String, serde_json::Value>),
    PickWallpaperFile,
    WallpaperPicked(String),
    CompleteOnboarding(String),
    RestoreOriginalDll,
    CheckUpdate,
    PerformStudioUpdate,
    StudioUpdateComplete,
    UpdateStatus {
        studio_has_update: bool,
        studio_latest: String,
        studio_current: Option<String>,
        launcher_new_version: Option<String>,
    },
    DownloadPlaceSnapshot {
        place_id: u64,
    },
    RefreshExperiences,
    PickLocalPlaceFile,
    FetchSubplaces {
        universe_id: u64,
    },
    SubplacesLoaded {
        universe_id: u64,
        subplaces_json: String,
    },
}

#[derive(Serialize)]
struct JsState<'a> {
    user: &'a Option<UserProfile>,
    places: &'a Vec<SavedPlace>,
    trending: &'a Vec<TrendingExperience>,
    config: &'a LauncherConfig,
    hardware: HardwareState,
    status_text: &'a str,
    is_running: bool,
    gpu_telemetry: Option<crate::runtime::GpuTelemetry>,
}

#[derive(Serialize)]
struct HardwareState {
    total_ram_mib: u64,
    cpu_cores: usize,
    gpu: String,
    is_igpu: bool,
    recommended_profile: String,
    has_wine: bool,
    wine_version: String,
}

fn get_embedded_html() -> String {
    let html = include_str!("../ui/index.html");
    let css = include_str!("../ui/style.css");
    let js = include_str!("../ui/app.js");

    html.replace(
        r#"<link rel="stylesheet" href="style.css">"#,
        &format!("<style>{}</style>", css),
    )
    .replace(
        r#"<script src="app.js"></script>"#,
        &format!("<script>{}</script>", js),
    )
}

pub fn run_gui() -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy: EventLoopProxy<UserEvent> = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("RTRUST - Roblox Studio Launcher")
        .with_inner_size(LogicalSize::new(1100.0, 680.0))
        .with_min_inner_size(LogicalSize::new(880.0, 540.0))
        .with_decorations(false)
        .with_transparent(false)
        .build(&event_loop)?;

    let html_content = get_embedded_html();

    let ipc_proxy = proxy.clone();
    let webview_builder = WebViewBuilder::new()
        .with_html(html_content)
        .with_ipc_handler(move |req| {
            let body = req.body();
            if let Ok(msg) = serde_json::from_str::<IncomingIpc>(body) {
                match msg {
                    IncomingIpc::DragWindow => {
                        let _ = ipc_proxy.send_event(UserEvent::DragWindow);
                    }
                    IncomingIpc::Minimize => {
                        let _ = ipc_proxy.send_event(UserEvent::Minimize);
                    }
                    IncomingIpc::Maximize => {
                        let _ = ipc_proxy.send_event(UserEvent::Maximize);
                    }
                    IncomingIpc::Close => {
                        let _ = ipc_proxy.send_event(UserEvent::Close);
                    }
                    IncomingIpc::StartWebviewLogin => {
                        let _ = ipc_proxy.send_event(UserEvent::StartWebviewLogin);
                    }
                    IncomingIpc::StartQuickLogin => {
                        let _ = ipc_proxy.send_event(UserEvent::StartQuickLogin);
                    }
                    IncomingIpc::OpenPluginsFolder => {
                        let _ = ipc_proxy.send_event(UserEvent::OpenPluginsFolder);
                    }
                    IncomingIpc::ImportPlugins => {
                        let _ = ipc_proxy.send_event(UserEvent::ImportPlugins);
                    }
                    IncomingIpc::OpenStudioFolder => {
                        let _ = ipc_proxy.send_event(UserEvent::OpenStudioFolder);
                    }
                    IncomingIpc::OpenPrefixFolder => {
                        let _ = ipc_proxy.send_event(UserEvent::OpenPrefixFolder);
                    }
                    IncomingIpc::SignOut => {
                        let _ = ipc_proxy.send_event(UserEvent::SignOut);
                    }
                    IncomingIpc::OpenUrl { url } => {
                        let _ = ipc_proxy.send_event(UserEvent::OpenUrl { url });
                    }
                    IncomingIpc::AddPlace { name, place_id } => {
                        let _ = ipc_proxy.send_event(UserEvent::AddPlace { name, place_id });
                    }
                    IncomingIpc::SaveConfig { config } => {
                        let _ = ipc_proxy.send_event(UserEvent::SaveConfig { config });
                    }
                    IncomingIpc::Play {
                        place_id,
                        fps,
                        force_xwayland,
                    } => {
                        let _ = ipc_proxy.send_event(UserEvent::Play {
                            place_id,
                            fps: fps.unwrap_or(60),
                            force_xwayland: force_xwayland.unwrap_or(true),
                        });
                    }
                    IncomingIpc::Init => {
                        let _ = ipc_proxy.send_event(UserEvent::SendStateToJs);
                    }
                    IncomingIpc::ApplyProfile { profile } => {
                        let _ = ipc_proxy.send_event(UserEvent::ApplyProfile(profile));
                    }
                    IncomingIpc::ResetRecommendedProfile => {
                        let _ = ipc_proxy.send_event(UserEvent::ResetRecommendedProfile);
                    }
                    IncomingIpc::SaveRawFflags { fflags } => {
                        let _ = ipc_proxy.send_event(UserEvent::SaveRawFflags(fflags));
                    }
                    IncomingIpc::PickWallpaperFile => {
                        let _ = ipc_proxy.send_event(UserEvent::PickWallpaperFile);
                    }
                    IncomingIpc::CompleteOnboarding { profile } => {
                        let _ = ipc_proxy.send_event(UserEvent::CompleteOnboarding(profile));
                    }
                    IncomingIpc::RestoreOriginalDll => {
                        let _ = ipc_proxy.send_event(UserEvent::RestoreOriginalDll);
                    }
                    IncomingIpc::CheckUpdate => {
                        let _ = ipc_proxy.send_event(UserEvent::CheckUpdate);
                    }
                    IncomingIpc::PerformStudioUpdate => {
                        let _ = ipc_proxy.send_event(UserEvent::PerformStudioUpdate);
                    }
                    IncomingIpc::DownloadPlaceSnapshot { place_id } => {
                        let _ = ipc_proxy.send_event(UserEvent::DownloadPlaceSnapshot { place_id });
                    }
                    IncomingIpc::RefreshExperiences => {
                        let _ = ipc_proxy.send_event(UserEvent::RefreshExperiences);
                    }
                    IncomingIpc::PickLocalPlaceFile => {
                        let _ = ipc_proxy.send_event(UserEvent::PickLocalPlaceFile);
                    }
                    IncomingIpc::FetchSubplaces { universe_id } => {
                        let _ = ipc_proxy.send_event(UserEvent::FetchSubplaces { universe_id });
                    }
                }
            }
        });

    #[cfg(not(target_os = "linux"))]
    let webview = webview_builder.build(&window)?;

    #[cfg(target_os = "linux")]
    let webview = {
        let vbox = window
            .default_vbox()
            .context("Failed to get default vbox from Tao window")?;
        let wv = webview_builder.build_gtk(vbox)?;
        vbox.show_all();
        wv
    };

    let mut config = LauncherConfig::load();
    if config.user_profile.is_none() {
        if let Some(saved) = UserProfile::load_session() {
            info!("Auto-loaded persistent session for @{}", saved.username);
            config.user_profile = Some(saved);
        }
    }
    let hardware = HardwareProfile::detect();
    let mut status_message = "Ready".to_string();
    let mut is_studio_running = false;
    let mut trending_experiences: Vec<TrendingExperience> = Vec::new();

    // Check for Roblox Studio and GitHub updates in background on start
    {
        let p_update = proxy.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new();
            if let Ok(runtime) = rt {
                runtime.block_on(async {
                    let api = RobloxApi::new();
                    let prefix_mgr = PrefixManager::new().ok();
                    let versions_dir = prefix_mgr.map(|p| p.versions_dir()).unwrap_or_else(|| {
                        dirs::data_local_dir()
                            .map(|d| d.join("rtrust/versions"))
                            .unwrap_or_default()
                    });

                    let mut studio_has_update = false;
                    let mut studio_latest = String::new();
                    let mut studio_current = None;

                    if let Ok(status) = api.check_studio_update(&versions_dir).await {
                        studio_has_update = status.has_update;
                        studio_latest = status.latest_version;
                        studio_current = status.current_version;
                    }

                    let launcher_new_version = api.check_github_release().await.unwrap_or(None);

                    let _ = p_update.send_event(UserEvent::UpdateStatus {
                        studio_has_update,
                        studio_latest,
                        studio_current,
                        launcher_new_version,
                    });
                });
            }
        });
    }

    let sync_state = |webview: &wry::WebView,
                      config: &LauncherConfig,
                      hardware: &HardwareProfile,
                      trending: &Vec<TrendingExperience>,
                      status: &str,
                      running: bool| {
        let recommended_profile = if hardware.is_constrained_memory() {
            "potato".to_string()
        } else if hardware.total_ram_mib <= 16384 {
            "balanced".to_string()
        } else {
            "high".to_string()
        };

        let (has_wine, wine_version) = {
            let wine_bin = PrefixManager::get_wine_binary();
            if let Ok(output) = Command::new(&wine_bin).arg("--version").output() {
                if output.status.success() {
                    (true, String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    (false, "Unknown".to_string())
                }
            } else {
                (false, "Not Found".to_string())
            }
        };

        let state = JsState {
            user: &config.user_profile,
            places: &config.saved_places,
            trending,
            config,
            hardware: HardwareState {
                total_ram_mib: hardware.total_ram_mib,
                cpu_cores: hardware.cpu_cores,
                gpu: format!("{:?}", hardware.primary_gpu),
                is_igpu: hardware.is_integrated_gpu,
                recommended_profile,
                has_wine,
                wine_version,
            },
            status_text: status,
            is_running: running,
            gpu_telemetry: crate::runtime::SlimRuntimeManager::read_gpu_telemetry(),
        };

        if let Ok(json) = serde_json::to_string(&state) {
            let script = format!("if (window.updateState) {{ window.updateState({}); }}", json);
            let _ = webview.evaluate_script(&script);
        }
    };

    let mut login_window: Option<tao::window::Window> = None;
    let mut login_webview: Option<wry::WebView> = None;

    event_loop.run(move |event, event_loop_target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                info!("Riot-Style WebUI Launcher Initialized");
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Some(ref lw) = login_window {
                    if lw.id() == window_id {
                        login_webview = None;
                        login_window = None;
                        return;
                    }
                }
                if window_id == window.id() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::DragWindow => {
                    let _ = window.drag_window();
                }
                UserEvent::Minimize => {
                    window.set_minimized(true);
                }
                UserEvent::Maximize => {
                    window.set_maximized(!window.is_maximized());
                }
                UserEvent::Close => {
                    *control_flow = ControlFlow::Exit;
                }
                UserEvent::StartWebviewLogin => {
                    if let Some(ref lw) = login_window {
                        lw.set_focus();
                    } else {
                        info!("Opening Roblox WebView login window...");
                        match WindowBuilder::new()
                            .with_title("Roblox Sign In - RtRust")
                            .with_inner_size(LogicalSize::new(480.0, 720.0))
                            .with_resizable(true)
                            .build(event_loop_target)
                        {
                            Ok(login_win) => {
                                #[cfg(target_os = "linux")]
                                let webview_res = {
                                    if let Some(vbox) = login_win.default_vbox() {
                                        let p_nav = proxy.clone();
                                        let p_load = proxy.clone();
                                        let wv_builder = WebViewBuilder::new()
                                            .with_url("https://www.roblox.com/login?returnUrl=https%3A%2F%2Fwww.roblox.com%2Fhome")
                                            .with_navigation_handler(move |url| {
                                                if url.contains("roblox.com/home") || url.contains("roblox.com/discover") || url.contains("roblox.com/users") {
                                                    let _ = p_nav.send_event(UserEvent::CheckWebviewCookies);
                                                }
                                                true
                                            })
                                            .with_on_page_load_handler(move |_, url| {
                                                if url.contains("roblox.com/home") || url.contains("roblox.com/discover") || url.contains("roblox.com/users") {
                                                    let _ = p_load.send_event(UserEvent::CheckWebviewCookies);
                                                }
                                            });
                                        let wv = wv_builder.build_gtk(vbox);
                                        vbox.show_all();
                                        wv
                                    } else {
                                        Err(wry::Error::InitScriptError)
                                    }
                                };
                                #[cfg(not(target_os = "linux"))]
                                let webview_res = WebViewBuilder::new()
                                    .with_url("https://www.roblox.com/login?returnUrl=https%3A%2F%2Fwww.roblox.com%2Fhome")
                                    .build(&login_win);

                                match webview_res {
                                    Ok(wv) => {
                                        login_webview = Some(wv);
                                        login_window = Some(login_win);

                                        // Periodic check for cookies every 2 seconds while login window is open
                                        let p_timer = proxy.clone();
                                        std::thread::spawn(move || {
                                            for _ in 0..90 {
                                                std::thread::sleep(std::time::Duration::from_secs(2));
                                                let _ = p_timer.send_event(UserEvent::CheckWebviewCookies);
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        error!("Failed to create login webview: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to create login window: {}", e);
                            }
                        }
                    }
                }
                UserEvent::CheckWebviewCookies => {
                    if let Some(ref wv) = login_webview {
                        let candidate_urls = [
                            "https://www.roblox.com",
                            "https://roblox.com",
                            "https://auth.roblox.com",
                            "https://apis.roblox.com",
                            "https://www.roblox.com/home",
                        ];

                        let mut found_cookie = None;
                        for target_url in &candidate_urls {
                            if let Ok(cookies) = wv.cookies_for_url(target_url) {
                                for c in cookies {
                                    if c.name() == ".ROBLOSECURITY" && !c.value().is_empty() {
                                        found_cookie = Some(c.value().to_string());
                                        break;
                                    }
                                }
                            }
                            if found_cookie.is_some() {
                                break;
                            }
                        }

                        if let Some(cookie_val) = found_cookie {
                            info!("Captured .ROBLOSECURITY cookie from Roblox WebView!");
                            if let Some(ref lw) = login_window {
                                lw.set_visible(false);
                            }
                            login_webview = None;
                            login_window = None;

                            let p_auth = proxy.clone();
                            std::thread::spawn(move || {
                                let rt = match tokio::runtime::Runtime::new() {
                                    Ok(r) => r,
                                    Err(e) => {
                                        error!("Runtime error: {}", e);
                                        return;
                                    }
                                };
                                rt.block_on(async {
                                    let api = RobloxApi::new();
                                    match api.authenticate_with_cookie(&cookie_val).await {
                                        Ok(user) => {
                                            if let Ok(places) = api.fetch_user_places(user.user_id, &cookie_val).await {
                                                if !places.is_empty() {
                                                    let mut cfg = LauncherConfig::load();
                                                    cfg.saved_places = places;
                                                    let _ = cfg.save();
                                                }
                                            }
                                            let _ = p_auth.send_event(UserEvent::LoginSuccess(user));
                                        }
                                        Err(e) => {
                                            error!("Failed to authenticate captured cookie: {}", e);
                                            let _ = p_auth.send_event(UserEvent::StatusUpdate(format!("Login failed: {}", e)));
                                        }
                                    }
                                });
                            });
                        }
                    }
                }
                UserEvent::StartQuickLogin => {
                    let p_quick = proxy.clone();
                    std::thread::spawn(move || {
                        let rt = match tokio::runtime::Runtime::new() {
                            Ok(r) => r,
                            Err(e) => {
                                error!("Runtime error: {}", e);
                                return;
                            }
                        };
                        rt.block_on(async {
                            let api = RobloxApi::new();
                            match api.create_quick_login().await {
                                Ok(data) => {
                                    let code = data.code.clone();
                                    let private_key = data.private_key.clone();
                                    let _ = p_quick.send_event(UserEvent::QuickLoginReady {
                                        code: data.code,
                                        qr_url: data.qr_url,
                                    });

                                    for _ in 0..100 {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                        match api.poll_quick_login_status(&code).await {
                                            Ok(status) => {
                                                if status == "Validated" {
                                                    info!("Quick login validated! Redeeming session...");
                                                    match api.redeem_quick_login(&code, &private_key).await {
                                                        Ok(cookie) => {
                                                            match api.authenticate_with_cookie(&cookie).await {
                                                                Ok(user) => {
                                                                    if let Ok(places) = api.fetch_user_places(user.user_id, &cookie).await {
                                                                        if !places.is_empty() {
                                                                            let mut cfg = LauncherConfig::load();
                                                                            cfg.saved_places = places;
                                                                            let _ = cfg.save();
                                                                        }
                                                                    }
                                                                    let _ = p_quick.send_event(UserEvent::LoginSuccess(user));
                                                                }
                                                                Err(e) => {
                                                                    error!("Auth failed after quick login: {}", e);
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            error!("Redeem quick login failed: {}", e);
                                                        }
                                                    }
                                                    break;
                                                } else if status == "Cancelled" || status == "Expired" {
                                                    info!("Quick login status: {}", status);
                                                    let _ = p_quick.send_event(UserEvent::StatusUpdate(format!("Quick login {}", status)));
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                tracing::debug!("Quick login poll: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to create quick login: {}", e);
                                    let _ = p_quick.send_event(UserEvent::QuickLoginError(e.to_string()));
                                }
                            }
                        });
                    });
                }
                UserEvent::QuickLoginReady { code, qr_url } => {
                    let script = format!(
                        "if (window.onQuickLoginCode) {{ window.onQuickLoginCode('{}', '{}', 300); }}",
                        code,
                        qr_url.unwrap_or_default()
                    );
                    let _ = webview.evaluate_script(&script);
                }
                UserEvent::QuickLoginError(err) => {
                    let safe_err = err.replace('\'', "\\'");
                    let script = format!(
                        "if (window.onQuickLoginError) {{ window.onQuickLoginError('{}'); }}",
                        safe_err
                    );
                    let _ = webview.evaluate_script(&script);
                }
                UserEvent::LoginSuccess(user) => {
                    let _ = user.save_session();
                    let display_name = user.display_name.clone();
                    config.user_profile = Some(user);
                    config.saved_places = LauncherConfig::load().saved_places;
                    let _ = config.save();
                    status_message = format!("Logged in as {}", display_name);
                    let _ = webview.evaluate_script("if (window.showHub) window.showHub();");
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::TrendingLoaded(items) => {
                    trending_experiences = items;
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::InstallProgress { current, total, pkg } => {
                    let safe_pkg = pkg.replace('\'', "\\'");
                    let script = format!(
                        "if (window.updateInstallProgress) {{ window.updateInstallProgress({}, {}, '{}'); }}",
                        current, total, safe_pkg
                    );
                    let _ = webview.evaluate_script(&script);
                }
                UserEvent::SendStateToJs => {
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::StatusUpdate(msg) => {
                    status_message = msg;
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::OpenUrl { url } => {
                    #[cfg(target_os = "windows")]
                    let _ = Command::new("cmd").args(["/C", "start", "", &url]).spawn();
                    #[cfg(not(target_os = "windows"))]
                    let _ = Command::new("xdg-open").arg(&url).spawn();
                }
                UserEvent::RestoreOriginalDll => {
                    let mut restored = false;
                    #[cfg(target_os = "windows")]
                    if let Some(exe) = Installer::find_existing_studio_windows() {
                        if let Some(dir) = exe.parent() {
                            if let Ok(()) = Installer::safe_swap_webview2(dir, false) {
                                restored = true;
                            }
                        }
                    }
                    if let Ok(prefix) = PrefixManager::new() {
                        let versions_dir = prefix.versions_dir();
                        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                            for entry in entries.flatten() {
                                if entry.path().is_dir() {
                                    if let Ok(()) = Installer::safe_swap_webview2(&entry.path(), false) {
                                        restored = true;
                                    }
                                }
                            }
                        }
                    }
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: if restored {
                            "Original WebView2Loader.dll successfully restored!".to_string()
                        } else {
                            "No backed up WebView2Loader.dll.bak found to restore.".to_string()
                        },
                        is_success: restored,
                    });
                }
                UserEvent::OpenPluginsFolder => {
                    if let Ok(prefix) = PrefixManager::new() {
                        match prefix.open_plugins_folder() {
                            Ok(path) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Opened Plugins: {}", path.display()),
                                    is_success: true,
                                });
                            }
                            Err(e) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Failed to open Plugins: {}", e),
                                    is_success: false,
                                });
                            }
                        }
                    }
                }
                UserEvent::ImportPlugins => {
                    let p_toast = proxy.clone();
                    std::thread::spawn(move || {
                        if let Some(files) = rfd::FileDialog::new()
                            .set_title("Select Roblox Plugins (*.rbxm, *.rbxmx, *.lua)")
                            .add_filter("Roblox Plugin", &["rbxm", "rbxmx", "lua"])
                            .pick_files()
                        {
                            if let Ok(prefix) = PrefixManager::new() {
                                match prefix.import_plugins(&files) {
                                    Ok(count) => {
                                        let _ = p_toast.send_event(UserEvent::ShowToast {
                                            message: format!("Successfully installed {} plugin(s)!", count),
                                            is_success: true,
                                        });
                                    }
                                    Err(e) => {
                                        let _ = p_toast.send_event(UserEvent::ShowToast {
                                            message: format!("Plugin import error: {}", e),
                                            is_success: false,
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
                UserEvent::OpenStudioFolder => {
                    if let Ok(prefix) = PrefixManager::new() {
                        match prefix.open_studio_folder() {
                            Ok(path) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Opened Studio folder: {}", path.display()),
                                    is_success: true,
                                });
                            }
                            Err(e) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Failed to open Studio folder: {}", e),
                                    is_success: false,
                                });
                            }
                        }
                    }
                }
                UserEvent::OpenPrefixFolder => {
                    if let Ok(prefix) = PrefixManager::new() {
                        match prefix.open_prefix_folder() {
                            Ok(path) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Opened Wine Prefix: {}", path.display()),
                                    is_success: true,
                                });
                            }
                            Err(e) => {
                                let _ = proxy.send_event(UserEvent::ShowToast {
                                    message: format!("Failed to open Wine prefix: {}", e),
                                    is_success: false,
                                });
                            }
                        }
                    }
                }
                UserEvent::ShowToast { message, is_success } => {
                    let safe_msg = message.replace('\'', "\\'").replace('\n', " ");
                    let script = format!(
                        "if (window.showToast) {{ window.showToast('{}', {}); }}",
                        safe_msg, is_success
                    );
                    let _ = webview.evaluate_script(&script);
                }
                UserEvent::SignOut => {
                    let _ = UserProfile::delete_session();
                    config.user_profile = None;
                    let _ = config.save();
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        "Signed out",
                        is_studio_running,
                    );
                }
                UserEvent::AddPlace { name, place_id } => {
                    config.saved_places.retain(|p| p.place_id != place_id);
                    config.saved_places.insert(
                        0,
                        SavedPlace {
                            name,
                            place_id,
                            description: "Custom Place".to_string(),
                            universe_id: None,
                            thumbnail_url: None,
                            team_create_enabled: None,
                            category: "custom".to_string(),
                            local_path: None,
                        },
                    );
                    let _ = config.save();
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::SaveConfig {
                    config: json_config,
                } => {
                    if let Ok(mut new_cfg) =
                        serde_json::from_value::<LauncherConfig>(json_config)
                    {
                        new_cfg.saved_places = config.saved_places.clone();
                        new_cfg.user_profile = config.user_profile.clone();
                        config = new_cfg;
                        let _ = config.save();
                        sync_state(
                            &webview,
                            &config,
                            &hardware,
                            &trending_experiences,
                            &status_message,
                            is_studio_running,
                        );
                    }
                }
                UserEvent::ApplyProfile(profile) => {
                    config.apply_profile(&profile, &hardware);
                    let _ = config.save();
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: format!("Switched to {} profile", profile.to_uppercase()),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::ResetRecommendedProfile => {
                    config.apply_profile("auto", &hardware);
                    let _ = config.save();
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: "Profile reset to recommended hardware settings!".to_string(),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::CompleteOnboarding(profile) => {
                    config.apply_profile(&profile, &hardware);
                    config.first_run_completed = true;
                    let _ = config.save();
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: "Welcome to RTRUST! Configuration ready.".to_string(),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::PickWallpaperFile => {
                    let p_toast = proxy.clone();
                    std::thread::spawn(move || {
                        if let Some(file) = rfd::FileDialog::new()
                            .set_title("Choose Wallpaper (Image or Video)")
                            .add_filter("Images & Videos", &["png", "jpg", "jpeg", "webp", "mp4", "webm"])
                            .pick_file()
                        {
                            let path_str = file.to_string_lossy().to_string();
                            let _ = p_toast.send_event(UserEvent::WallpaperPicked(path_str));
                        }
                    });
                }
                UserEvent::WallpaperPicked(path) => {
                    config.wallpaper_mode = "local".to_string();
                    config.wallpaper_path = Some(path);
                    let _ = config.save();
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: "Custom wallpaper updated successfully!".to_string(),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::SaveRawFflags(map) => {
                    config.raw_custom_fflags = map;
                    let _ = config.save();
                    if let Ok(prefix) = PrefixManager::new() {
                        if let Ok(entries) = std::fs::read_dir(prefix.versions_dir()) {
                            for entry in entries.flatten() {
                                if entry.path().is_dir() {
                                    let _ = FFlagsGenerator::inject_settings(&entry.path(), &config);
                                }
                            }
                        }
                    }
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: "Custom FFlags merged and saved to ClientSettings.json!".to_string(),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::Play {
                    place_id,
                    fps,
                    force_xwayland,
                } => {
                    config.target_fps = fps;
                    config.force_xwayland_mouse = force_xwayland;
                    config.place_id = place_id;
                    let _ = config.save();

                    is_studio_running = true;
                    status_message = "Preparing Roblox Studio...".to_string();
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );

                    let p_clone = proxy.clone();
                    let conf_clone = config.clone();
                    let prof_clone = hardware.clone();

                    std::thread::spawn(move || {
                        let rt = match tokio::runtime::Runtime::new() {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = p_clone.send_event(UserEvent::StatusUpdate(format!(
                                    "Runtime Error: {}",
                                    e
                                )));
                                let _ = p_clone.send_event(UserEvent::StudioExited(-1));
                                return;
                            }
                        };

                        let progress_proxy = p_clone.clone();
                        let res = rt.block_on(async {
                            run_studio_session(&conf_clone, &prof_clone, progress_proxy).await
                        });

                        match res {
                            Ok(code) => {
                                let _ = p_clone.send_event(UserEvent::StudioExited(code));
                            }
                            Err(e) => {
                                let _ = p_clone.send_event(UserEvent::StatusUpdate(format!(
                                    "Launch Error: {}",
                                    e
                                )));
                                let _ = p_clone.send_event(UserEvent::StudioExited(-1));
                            }
                        }
                    });
                }
                UserEvent::StudioStarted => {
                    is_studio_running = true;
                    status_message = "Studio Running".to_string();
                    let _ = webview.evaluate_script("if (window.finishInstallProgress) { window.finishInstallProgress(); }");
                    if config.auto_hide_launcher {
                        window.set_visible(false);
                    }
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::StudioExited(code) => {
                    is_studio_running = false;
                    status_message = format!("Studio exited (code {})", code);
                    let _ = webview.evaluate_script("if (window.finishInstallProgress) { window.finishInstallProgress(); }");
                    window.set_visible(true);
                    window.set_focus();
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::UpdateStatus {
                    studio_has_update,
                    studio_latest,
                    studio_current,
                    launcher_new_version,
                } => {
                    let status_obj = serde_json::json!({
                        "has_update": studio_has_update,
                        "latest_version": studio_latest,
                        "current_version": studio_current,
                        "launcher_update": launcher_new_version,
                    });
                    let script = format!(
                        "if (window.setUpdateAvailable) {{ window.setUpdateAvailable({}); }}",
                        status_obj
                    );
                    let _ = webview.evaluate_script(&script);
                }
                UserEvent::CheckUpdate => {
                    let p_update = proxy.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new();
                        if let Ok(runtime) = rt {
                            runtime.block_on(async {
                                let api = RobloxApi::new();
                                let prefix_mgr = PrefixManager::new().ok();
                                let versions_dir = prefix_mgr.map(|p| p.versions_dir()).unwrap_or_else(|| {
                                    dirs::data_local_dir()
                                        .map(|d| d.join("rtrust/versions"))
                                        .unwrap_or_default()
                                });

                                let mut studio_has_update = false;
                                let mut studio_latest = String::new();
                                let mut studio_current = None;

                                if let Ok(status) = api.check_studio_update(&versions_dir).await {
                                    studio_has_update = status.has_update;
                                    studio_latest = status.latest_version;
                                    studio_current = status.current_version;
                                }

                                let launcher_new_version = api.check_github_release().await.unwrap_or(None);

                                let _ = p_update.send_event(UserEvent::UpdateStatus {
                                    studio_has_update,
                                    studio_latest,
                                    studio_current,
                                    launcher_new_version,
                                });
                            });
                        }
                    });
                }
                UserEvent::PerformStudioUpdate => {
                    is_studio_running = true;
                    status_message = "Updating Roblox Studio...".to_string();
                    let _ = webview.evaluate_script("if (window.onStudioUpdateStarted) { window.onStudioUpdateStarted(); }");
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );

                    let p_clone = proxy.clone();
                    let conf_clone = config.clone();
                    let prof_clone = hardware.clone();

                    std::thread::spawn(move || {
                        let rt = match tokio::runtime::Runtime::new() {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = p_clone.send_event(UserEvent::StatusUpdate(format!("Update runtime error: {}", e)));
                                let _ = p_clone.send_event(UserEvent::StudioExited(-1));
                                return;
                            }
                        };

                        let progress_proxy = p_clone.clone();
                        let res = rt.block_on(async {
                            let prefix_mgr = Arc::new(PrefixManager::new()?);
                            prefix_mgr.setup_prefix(&conf_clone.mouse_warp_mode)?;
                            let _ = crate::runtime::SlimRuntimeManager::setup_slim_prefix(&prefix_mgr.prefix_dir, &conf_clone.mouse_warp_mode);

                            let installer = Installer::new(prefix_mgr.clone());
                            let p_cb = progress_proxy.clone();
                            let cb = Arc::new(move |cur, tot, pkg: &str| {
                                let _ = p_cb.send_event(UserEvent::InstallProgress {
                                    current: cur,
                                    total: tot,
                                    pkg: pkg.to_string(),
                                });
                            });

                            installer.ensure_latest_studio_with_progress(&prof_clone, &conf_clone, Some(cb)).await
                        });

                        match res {
                            Ok(exe) => {
                                info!("Studio update and configuration successful: {:?}", exe);
                                let _ = p_clone.send_event(UserEvent::StudioUpdateComplete);
                            }
                            Err(e) => {
                                error!("Studio update failed: {}", e);
                                let _ = p_clone.send_event(UserEvent::StatusUpdate(format!("Update failed: {}", e)));
                                let _ = p_clone.send_event(UserEvent::StudioExited(-1));
                            }
                        }
                    });
                }
                UserEvent::StudioUpdateComplete => {
                    is_studio_running = false;
                    status_message = "Roblox Studio is up to date (Ready)".to_string();
                    let _ = webview.evaluate_script("if (window.finishInstallProgress) { window.finishInstallProgress(); }");
                    let _ = webview.evaluate_script("if (window.setUpdateAvailable) { window.setUpdateAvailable({ has_update: false, latest_version: '', current_version: '', launcher_update: null }); }");
                    let _ = proxy.send_event(UserEvent::ShowToast {
                        message: "Roblox Studio successfully updated and compiled!".to_string(),
                        is_success: true,
                    });
                    sync_state(
                        &webview,
                        &config,
                        &hardware,
                        &trending_experiences,
                        &status_message,
                        is_studio_running,
                    );
                }
                UserEvent::DownloadPlaceSnapshot { place_id } => {
                    let p_down = proxy.clone();
                    let cookie_opt = config.user_profile.as_ref().map(|u| u.roblosecurity.clone());
                    std::thread::spawn(move || {
                        if let Some(cookie) = cookie_opt {
                            let rt = tokio::runtime::Runtime::new();
                            if let Ok(runtime) = rt {
                                runtime.block_on(async {
                                    let api = RobloxApi::new();
                                    match api.download_place_snapshot(place_id, &cookie).await {
                                        Ok(path) => {
                                            let path_str = path.to_string_lossy().to_string();
                                            let mut cfg = LauncherConfig::load();
                                            if let Some(p) = cfg.saved_places.iter_mut().find(|p| p.place_id == place_id) {
                                                p.local_path = Some(path_str.clone());
                                                let _ = cfg.save();
                                            }
                                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                            let _ = p_down.send_event(UserEvent::ShowToast {
                                                message: format!("Saved snapshot: {}", filename),
                                                is_success: true,
                                            });
                                            let _ = p_down.send_event(UserEvent::SendStateToJs);
                                        }
                                        Err(e) => {
                                            let _ = p_down.send_event(UserEvent::ShowToast {
                                                message: format!("Download failed: {}", e),
                                                is_success: false,
                                            });
                                        }
                                    }
                                });
                            }
                        } else {
                            let _ = p_down.send_event(UserEvent::ShowToast {
                                message: "Please log in to download place snapshots".to_string(),
                                is_success: false,
                            });
                        }
                    });
                }
                UserEvent::RefreshExperiences => {
                    let p_exp = proxy.clone();
                    if let Some(ref prof) = config.user_profile {
                        let uid = prof.user_id;
                        let cookie = prof.roblosecurity.clone();
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new();
                            if let Ok(runtime) = rt {
                                runtime.block_on(async {
                                    let api = RobloxApi::new();
                                    if let Ok(places) = api.fetch_all_user_experiences(uid, &cookie).await {
                                        if !places.is_empty() {
                                            let mut cfg = LauncherConfig::load();
                                            cfg.saved_places = places;
                                            let _ = cfg.save();
                                            let _ = p_exp.send_event(UserEvent::SendStateToJs);
                                            let _ = p_exp.send_event(UserEvent::ShowToast {
                                                message: "Experiences synced from Roblox Cloud!".to_string(),
                                                is_success: true,
                                            });
                                        }
                                    }
                                });
                            }
                        });
                    }
                }
                UserEvent::PickLocalPlaceFile => {
                    let p_clone = proxy.clone();
                    std::thread::spawn(move || {
                        if let Some(file) = rfd::FileDialog::new()
                            .set_title("Open Roblox Place File (.rbxl / .rbxlx)")
                            .add_filter("Roblox Place", &["rbxl", "rbxlx"])
                            .pick_file()
                        {
                            let path_str = file.to_string_lossy().to_string();
                            let name = file
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            
                            // Deterministic hash ID based on path
                            let mut h: u64 = 5381;
                            for b in path_str.bytes() {
                                h = ((h << 5).wrapping_add(h)).wrapping_add(b as u64);
                            }
                            let place_id = (h & 0x7FFFFFFF) + 1_000_000_000;

                            let new_place = SavedPlace {
                                name: format!("Local: {}", name),
                                place_id,
                                description: format!("Local File: {}", path_str),
                                universe_id: None,
                                thumbnail_url: None,
                                team_create_enabled: None,
                                category: "local".to_string(),
                                local_path: Some(path_str),
                            };

                            let mut cfg = LauncherConfig::load();
                            cfg.saved_places.retain(|p| p.local_path.as_ref() != new_place.local_path.as_ref());
                            cfg.saved_places.insert(0, new_place);
                            cfg.place_id = Some(place_id);
                            let _ = cfg.save();

                            let _ = p_clone.send_event(UserEvent::ShowToast {
                                message: format!("Loaded local place: {}", name),
                                is_success: true,
                            });
                            let _ = p_clone.send_event(UserEvent::SendStateToJs);
                        }
                    });
                }
                UserEvent::FetchSubplaces { universe_id } => {
                    let cookie_opt = config.user_profile.as_ref().map(|u| u.roblosecurity.clone());
                    let p_sub = proxy.clone();
                    std::thread::spawn(move || {
                        if let Some(cookie) = cookie_opt {
                            let rt = tokio::runtime::Runtime::new();
                            if let Ok(runtime) = rt {
                                runtime.block_on(async {
                                    let api = RobloxApi::new();
                                    if let Ok(subplaces) = api.fetch_universe_subplaces(universe_id, &cookie).await {
                                        if let Ok(json) = serde_json::to_string(&subplaces) {
                                            let _ = p_sub.send_event(UserEvent::SubplacesLoaded {
                                                universe_id,
                                                subplaces_json: json,
                                            });
                                        }
                                    }
                                });
                            }
                        }
                    });
                }
                UserEvent::SubplacesLoaded { universe_id, subplaces_json } => {
                    let safe_json = subplaces_json.replace('\\', "\\\\").replace('\'', "\\'");
                    let script = format!("if (window.onSubplacesLoaded) window.onSubplacesLoaded({}, '{}');", universe_id, safe_json);
                    let _ = webview.evaluate_script(&script);
                }
            },
            _ => (),
        }
    });
}

async fn run_studio_session(
    config: &LauncherConfig,
    profile: &HardwareProfile,
    progress_proxy: EventLoopProxy<UserEvent>,
) -> Result<i32> {
    #[cfg(target_os = "windows")]
    {
        run_studio_session_windows(config, profile, progress_proxy).await
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_studio_session_linux(config, profile, progress_proxy).await
    }
}

#[cfg(target_os = "windows")]
async fn run_studio_session_windows(
    config: &LauncherConfig,
    profile: &HardwareProfile,
    progress_proxy: EventLoopProxy<UserEvent>,
) -> Result<i32> {
    let prefix_mgr = Arc::new(PrefixManager::new()?);

    // 1. Check for existing native Windows installation of Roblox Studio
    let studio_exe = if let Some(existing) = Installer::find_existing_studio_windows() {
        info!("Using detected native Windows Roblox Studio: {:?}", existing);
        let version_dir = existing.parent().context("Invalid studio exe directory")?;
        // Inject ClientSettings FFlags
        let _ = crate::fflags::FFlagsGenerator::inject_settings(version_dir, config);
        existing
    } else {
        // Download/verify latest studio release
        let installer = Installer::new(prefix_mgr.clone());
        let p_cb = progress_proxy.clone();
        let cb = Arc::new(move |cur, tot, pkg: &str| {
            let _ = p_cb.send_event(UserEvent::InstallProgress {
                current: cur,
                total: tot,
                pkg: pkg.to_string(),
            });
        });
        installer
            .ensure_latest_studio_with_progress(profile, config, Some(cb))
            .await?
    };

    let studio_dir = studio_exe.parent().context("Invalid studio exe path")?;

    // 2. Safe DLL Swap:
    // If low-RAM / potato mode AND block_webview2 is active, swap in the embedded stub DLL.
    // Otherwise, restore original WebView2Loader.dll from .bak if backup exists.
    let potato_mode = config.profile_mode == "potato" && config.block_webview2;
    Installer::safe_swap_webview2(studio_dir, potato_mode)?;

    // Deploy or restore Rust DXGI Proxy Shim for GPU/VRAM control
    let _ = crate::runtime::SlimRuntimeManager::deploy_dxgi_shim(
        studio_dir,
        config.use_dxgi_shim,
        config.alt_tab_gpu_sleep,
        config.vram_cap_mb,
    );

    // Signal studio started
    let _ = progress_proxy.send_event(UserEvent::StudioStarted);

    // Spawn IPC server in background if not already running
    tokio::spawn(async move {
        let server = IpcServer::new();
        if let Err(e) = server.run().await {
            error!("IPC server error: {}", e);
        }
    });

    info!("Starting Roblox Studio native executable: {:?}", studio_exe);
    let mut cmd = Command::new(&studio_exe);
    cmd.current_dir(studio_dir);
    cmd.arg("-ide");

    // Custom user CLI flags
    if let Some(ref flags) = config.custom_cli_flags {
        for flag in flags.split_whitespace() {
            cmd.arg(flag);
        }
    }

    // Custom environment variables
    if let Some(ref env_str) = config.custom_env_vars {
        for token in env_str.split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                cmd.env(k, v);
            }
        }
    }

    // Single Sign-On: Request auth ticket if user is logged in
    if let Some(ref user) = config.user_profile {
        if !user.roblosecurity.is_empty() {
            info!("Requesting auth ticket for single sign-on as @{}...", user.username);
            let api = RobloxApi::new();
            match api.get_auth_ticket(&user.roblosecurity).await {
                Ok(ticket) => {
                    info!("Single sign-on ticket acquired successfully");
                    cmd.args(["-authTicket", &ticket]);
                }
                Err(e) => {
                    error!("Failed to obtain auth ticket: {}. Studio will prompt for manual login.", e);
                }
            }
        }
    }

    // Place Launch Modes: Mode 1 (Cloud Edit Protocol) vs Mode 2 (Local file)
    let selected_place = config
        .place_id
        .and_then(|pid| config.saved_places.iter().find(|p| p.place_id == pid));

    if let Some(place) = selected_place {
        if let Some(ref local_path) = place.local_path {
            info!("Launching Roblox Studio with local file: {}", local_path);
            cmd.arg(local_path);
        } else {
            let universe_id = place.universe_id.unwrap_or(place.place_id);
            let launch_protocol = format!(
                "roblox-studio:1+launchmode:edit+task:EditPlace+placeId:{}+universeId:{}+robloxLocale:en_us+gameLocale:en_us",
                place.place_id, universe_id
            );
            cmd.arg(launch_protocol);
            cmd.args(["-placeId", &place.place_id.to_string()]);
            if let Some(uid) = place.universe_id {
                cmd.args(["-universeId", &uid.to_string()]);
            }
        }
    } else if let Some(id) = config.place_id {
        cmd.args(["-placeId", &id.to_string()]);
    }

    // Low-RAM / Potato background cleaner for Windows
    let clean_potato = potato_mode;
    let stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_signal_thread = stop_signal.clone();

    if clean_potato {
        std::thread::spawn(move || {
            while !stop_signal_thread.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let _ = Command::new("taskkill")
                    .args(["/F", "/IM", "msedgewebview2.exe"])
                    .output();
                let _ = Command::new("taskkill")
                    .args(["/F", "/IM", "RobloxCrashHandler.exe"])
                    .output();
            }
        });
    }

    let status = cmd
        .status()
        .context("Failed to execute RobloxStudioBeta.exe")?;

    stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);

    Ok(status.code().unwrap_or(0))
}

#[cfg(not(target_os = "windows"))]
async fn run_studio_session_linux(
    config: &LauncherConfig,
    profile: &HardwareProfile,
    progress_proxy: EventLoopProxy<UserEvent>,
) -> Result<i32> {
    let prefix_mgr = Arc::new(PrefixManager::new()?);

    // Setup prefix (registry overrides, DXVK, Wayland pointer confinement)
    prefix_mgr.setup_prefix(&config.mouse_warp_mode)?;
    let _ = crate::runtime::SlimRuntimeManager::setup_slim_prefix(&prefix_mgr.prefix_dir, &config.mouse_warp_mode);

    // Download / verify latest studio release with progress reporting
    let installer = Installer::new(prefix_mgr.clone());
    let p_cb = progress_proxy.clone();
    let cb = Arc::new(move |cur, tot, pkg: &str| {
        let _ = p_cb.send_event(UserEvent::InstallProgress {
            current: cur,
            total: tot,
            pkg: pkg.to_string(),
        });
    });

    let studio_exe = installer
        .ensure_latest_studio_with_progress(profile, config, Some(cb))
        .await?;

    // Signal studio started
    let _ = progress_proxy.send_event(UserEvent::StudioStarted);

    // Spawn IPC server in background if not already running
    tokio::spawn(async move {
        let server = IpcServer::new();
        if let Err(e) = server.run().await {
            error!("IPC server error: {}", e);
        }
    });

    let cache_dir = prefix_mgr.data_dir.join("cache");
    let _ = std::fs::create_dir_all(&cache_dir);

    let wine_bin = if let Some(ref custom) = config.custom_runner_path {
        let p = std::path::PathBuf::from(custom.trim());
        if !custom.trim().is_empty() && (p.exists() || which_command_exists(custom.trim())) {
            info!("Using custom runner specified by user: {:?}", p);
            p
        } else {
            PrefixManager::get_wine_binary()
        }
    } else {
        PrefixManager::get_wine_binary()
    };

    info!("Starting Roblox Studio under Wine binary: {:?}", wine_bin);
    let studio_dir = studio_exe.parent().context("Invalid studio exe path")?;

    // Deploy or restore Rust DXGI Proxy Shim for GPU/VRAM control
    let _ = crate::runtime::SlimRuntimeManager::deploy_dxgi_shim(
        studio_dir,
        config.use_dxgi_shim,
        config.alt_tab_gpu_sleep,
        config.vram_cap_mb,
    );

    let mut args: Vec<String> = Vec::new();
    if config.virtual_desktop {
        args.push("explorer".to_string());
        args.push(format!("/desktop=Studio,{}", config.virtual_desktop_res));
    }

    args.push(studio_exe.to_string_lossy().to_string());
    args.push("-ide".to_string());

    // Custom user CLI flags
    if let Some(ref flags) = config.custom_cli_flags {
        for flag in flags.split_whitespace() {
            args.push(flag.to_string());
        }
    }

    // Single Sign-On: Request auth ticket if user is logged in
    if let Some(ref user) = config.user_profile {
        if !user.roblosecurity.is_empty() {
            info!("Requesting auth ticket for single sign-on as @{}...", user.username);
            let api = RobloxApi::new();
            match api.get_auth_ticket(&user.roblosecurity).await {
                Ok(ticket) => {
                    info!("Single sign-on ticket acquired successfully");
                    args.push("-authTicket".to_string());
                    args.push(ticket);
                }
                Err(e) => {
                    error!("Failed to obtain auth ticket: {}. Studio will prompt for manual login.", e);
                }
            }
        }
    }

    // Place Launch Modes: Mode 1 (Cloud Edit Protocol) vs Mode 2 (Local Snapshot/File)
    let selected_place = config
        .place_id
        .and_then(|pid| config.saved_places.iter().find(|p| p.place_id == pid));

    if let Some(place) = selected_place {
        if let Some(ref local_path) = place.local_path {
            info!("Launching Roblox Studio with local place snapshot: {}", local_path);
            let win_path = format!("Z:{}", local_path.replace('/', "\\"));
            args.push(win_path);
        } else {
            let universe_id = place.universe_id.unwrap_or(place.place_id);
            let launch_protocol = format!(
                "roblox-studio:1+launchmode:edit+task:EditPlace+placeId:{}+universeId:{}+robloxLocale:en_us+gameLocale:en_us",
                place.place_id, universe_id
            );
            args.push(launch_protocol);
            args.push("-placeId".to_string());
            args.push(place.place_id.to_string());
            if let Some(uid) = place.universe_id {
                args.push("-universeId".to_string());
                args.push(uid.to_string());
            }
        }
    } else if let Some(id) = config.place_id {
        args.push("-placeId".to_string());
        args.push(id.to_string());
    }

    // Wrap with Linux cgroups v2 resource scope if configured
    let mut cmd = crate::runtime::SlimRuntimeManager::build_launch_command(
        &wine_bin,
        &args,
        config.cgroup_ram_limit_gb,
        100,
    );

    cmd.current_dir(studio_dir);
    cmd.env("WINEPREFIX", &prefix_mgr.prefix_dir);
    cmd.env("WINEDEBUG", "-all");
    cmd.env("WINEFSYNC", "1");
    cmd.env("WINEESYNC", "1");
    cmd.env("WINEDLLOVERRIDES", "winemenubuilder.exe=d;msedgewebview2.exe=d;dxgi=n,b;d3d11=n,b");
    cmd.env("DXVK_HUD", "0");
    cmd.env("DXVK_STATE_CACHE", "1");
    cmd.env("DXVK_STATE_CACHE_PATH", &cache_dir);
    cmd.env("ANV_ENABLE_PIPELINE_CACHE", "1");
    cmd.env("MESA_VK_WSI_PRESENT_MODE", "mailbox");
    cmd.env("QT_QPA_PLATFORM_PLUGIN_PATH", "Qt5/platforms");

    // Wine mouse & pointer confinement tweaks
    cmd.env("WINE_MOUSE_WARP_OVERRIDE", "force");
    cmd.env("WINE_X11_WARP_POINTER", "1");
    cmd.env("WINE_VK_WAYLAND", "0");

    // Inject custom environment variables if specified
    if let Some(ref env_str) = config.custom_env_vars {
        for token in env_str.split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                cmd.env(k, v);
            }
        }
    }

    // Prepend wine's bin dir to PATH if a custom wine binary is used
    if let Some(parent) = wine_bin.parent() {
        if let Ok(current_path) = std::env::var("PATH") {
            cmd.env("PATH", format!("{}:{}", parent.display(), current_path));
        }
    }

    // Wayland camera fix
    if config.force_xwayland_mouse {
        cmd.env_remove("WAYLAND_DISPLAY");
    }

    // Low-RAM / Potato process cleaner daemon
    let clean_potato = config.profile_mode == "potato" && config.block_webview2;
    let stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_signal_thread = stop_signal.clone();

    if clean_potato {
        std::thread::spawn(move || {
            while !stop_signal_thread.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let _ = Command::new("pkill").arg("-f").arg("msedgewebview2.exe").output();
                let _ = Command::new("pkill").arg("-f").arg("RobloxCrashHandler.exe").output();
            }
        });
    }

    let status = cmd
        .status()
        .context("Failed to execute Wine process. Ensure 'wine' is installed on your PATH.")?;

    stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);

    Ok(status.code().unwrap_or(0))
}

#[allow(dead_code)]
fn which_command_exists(cmd: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("where")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
