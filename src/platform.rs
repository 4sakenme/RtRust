//! Cross-platform OS abstraction for Desktop, Android, and iOS.
//!
//! Centralizes asset loading, writable app data/cache directories, external URL
//! opening, safe-area insets (notch / home indicator), bundled Vietnamese font
//! fallback, and app lifecycle pause/resume state.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Bundled Vietnamese-capable TrueType font (`DejaVuSans.ttf`, full Latin
/// Extended-A/B/Additional coverage) embedded directly so `egui` never depends
/// on Android-only `/system/fonts/Roboto-Regular.ttf`.
pub const BUNDLED_VIETNAMESE_FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

static APP_PAUSED: AtomicBool = AtomicBool::new(false);

/// Called from `ApplicationHandler::suspended` when iOS/Android backgrounds the app.
pub fn on_app_paused() {
    APP_PAUSED.store(true, Ordering::Release);
    log::info!("[lifecycle] app paused/suspended");
}

/// Called from `ApplicationHandler::resumed` when iOS/Android foregrounds the app.
pub fn on_app_resumed() {
    APP_PAUSED.store(false, Ordering::Release);
    #[cfg(target_os = "ios")]
    enable_ios_multitouch();
    log::info!("[lifecycle] app resumed");
}

/// Returns `true` while the app is suspended/backgrounded.
#[inline]
pub fn is_app_paused() -> bool {
    APP_PAUSED.load(Ordering::Acquire)
}

/// Loads a bundled asset from `rel_path` (e.g. `"assets/terrain-materials.json"`).
///
/// - **iOS:** resolves relative to the `.app` bundle directory (`current_exe().parent()`).
/// - **Android:** checks `app_data_dir()`, `current_exe().parent()`, and `rel_path`.
/// - **Desktop:** checks `current_exe().parent()`, `CARGO_MANIFEST_DIR`, and `rel_path`.
pub fn load_asset_bytes(rel_path: &str) -> std::io::Result<Vec<u8>> {
    let rel = rel_path.trim_start_matches('/');
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(5);

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(rel));
            candidates.push(dir.join("../Resources").join(rel));
        }
    }

    #[cfg(target_os = "android")]
    {
        candidates.push(app_data_dir().join(rel));
    }

    candidates.push(PathBuf::from(rel));
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel));

    let mut last_err = None;
    for path in candidates {
        match std::fs::read(&path) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => last_err = Some(e),
        }
    }

    Err(last_err.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, format!("asset not found: {rel_path}"))
    }))
}

/// Returns the writable app data directory, creating it if missing.
///
/// - **iOS:** `$HOME/Documents` inside the app sandbox container.
/// - **Android:** `rustudio_core::paths::data()` (backed by `internal_data_path`).
/// - **Desktop:** `rustudio_core::paths::data()`.
pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        let dir = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Documents");
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = rustudio_core::paths::data();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

/// Returns the writable app cache directory, creating it if missing.
///
/// - **iOS:** `$HOME/Library/Caches` inside the app sandbox container.
/// - **Android / Desktop:** `rustudio_core::paths::cache()`.
pub fn app_cache_dir() -> PathBuf {
    #[cfg(target_os = "ios")]
    {
        let dir = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Caches");
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    #[cfg(not(target_os = "ios"))]
    {
        let dir = rustudio_core::paths::cache();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

/// Opens an external HTTP/HTTPS URL in the platform's default browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "ios")]
    {
        ios_open_url(url);
        return;
    }
    #[cfg(target_os = "android")]
    {
        android_open_url(url);
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
        return;
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Opens a local file or folder in the desktop file manager (no-op on iOS/Android).
pub fn open_path(path: &Path) -> std::io::Result<()> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    {
        let _ = path;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
        Ok(())
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
        Ok(())
    }
}

/// Returns `(top, bottom, left, right)` safe area insets in logical points so
/// HUD buttons and top/bottom bars pad cleanly inside notches, Dynamic Islands,
/// and home bars.
pub fn get_safe_area_insets() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "ios")]
    {
        enable_ios_multitouch();
        ios_safe_area_insets()
    }
    #[cfg(not(target_os = "ios"))]
    {
        (0.0, 0.0, 0.0, 0.0)
    }
}

/// Returns the current soft keyboard height in logical points (`dp` / `pt`)
/// when a text editor is active, so the Luau quick-insert / cursor bar floats
/// cleanly above the keyboard on both iOS and Android.
pub fn keyboard_height_points(ctx: &egui::Context) -> f32 {
    #[cfg(target_os = "android")]
    {
        crate::android_ime::keyboard_height_dp(ctx)
    }
    #[cfg(target_os = "ios")]
    {
        ios_keyboard_height_points(ctx)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = ctx;
        0.0
    }
}

/// Ensures every `UIWindow` and `UIView` in the active iOS view hierarchy has
/// `multipleTouchEnabled = YES` so simultaneous thumbstick + camera orbit + jump
/// touches are delivered independently without dropping the second finger.
#[cfg(target_os = "ios")]
pub fn enable_ios_multitouch() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    unsafe fn enable_view_tree(view: *mut AnyObject) {
        if view.is_null() {
            return;
        }
        let _: () = msg_send![view, setMultipleTouchEnabled: true];
        let subviews: *mut AnyObject = msg_send![view, subviews];
        if !subviews.is_null() {
            let count: usize = msg_send![subviews, count];
            for i in 0..count {
                let child: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
                enable_view_tree(child);
            }
        }
    }

    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let windows: *mut AnyObject = msg_send![app, windows];
        if windows.is_null() {
            return;
        }
        let count: usize = msg_send![windows, count];
        for idx in 0..count {
            let win: *mut AnyObject = msg_send![windows, objectAtIndex: idx];
            if !win.is_null() {
                enable_view_tree(win);
            }
        }
    }
}

#[cfg(target_os = "ios")]
fn ios_open_url(url: &str) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::{NSString, NSURL};

    let ns_str = NSString::from_str(url);
    let Some(ns_url) = NSURL::URLWithString(&ns_str) else {
        return;
    };
    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if !app.is_null() {
            let options: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
            let completion: *const AnyObject = std::ptr::null();
            let _: () = msg_send![app, openURL: &*ns_url, options: options, completionHandler: completion];
        }
    }
}

#[cfg(target_os = "ios")]
fn ios_safe_area_insets() -> (f32, f32, f32, f32) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send, Encode, Encoding};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UIEdgeInsets {
        top: f64,
        left: f64,
        bottom: f64,
        right: f64,
    }

    unsafe impl Encode for UIEdgeInsets {
        const ENCODING: Encoding = Encoding::Struct(
            "UIEdgeInsets",
            &[f64::ENCODING, f64::ENCODING, f64::ENCODING, f64::ENCODING],
        );
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    let mut top = 0.0_f32;
    let mut bottom = 0.0_f32;
    let mut left = 0.0_f32;
    let mut right = 0.0_f32;
    let mut screen_w = 375.0_f32;
    let mut screen_h = 812.0_f32;

    unsafe {
        let screen: *mut AnyObject = msg_send![class!(UIScreen), mainScreen];
        if !screen.is_null() {
            let bounds: CGRect = msg_send![screen, bounds];
            if bounds.size.width > 0.0 && bounds.size.height > 0.0 {
                screen_w = bounds.size.width as f32;
                screen_h = bounds.size.height as f32;
            }
        }

        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if !app.is_null() {
            let windows: *mut AnyObject = msg_send![app, windows];
            if !windows.is_null() {
                let count: usize = msg_send![windows, count];
                for idx in 0..count {
                    let win: *mut AnyObject = msg_send![windows, objectAtIndex: idx];
                    if win.is_null() {
                        continue;
                    }
                    let is_key: bool = msg_send![win, isKeyWindow];
                    if is_key || idx == 0 {
                        let insets: UIEdgeInsets = msg_send![win, safeAreaInsets];
                        top = (insets.top as f32).max(0.0);
                        bottom = (insets.bottom as f32).max(0.0);
                        left = (insets.left as f32).max(0.0);
                        right = (insets.right as f32).max(0.0);
                        if is_key {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Guarantee hardware notch / Dynamic Island & Home Indicator safe margins on
    // all-screen iPhones (iPhone X / 11 Pro / 12..16 where max dimension >= 812pt),
    // even when status bar is hidden.
    let max_dim = screen_w.max(screen_h);
    if max_dim >= 800.0 {
        if screen_h >= screen_w {
            // Portrait
            top = top.max(44.0);
            bottom = bottom.max(34.0);
        } else {
            // Landscape
            left = left.max(44.0);
            right = right.max(44.0);
            bottom = bottom.max(21.0);
        }
    }

    (top, bottom, left, right)
}

#[cfg(target_os = "ios")]
fn ios_keyboard_height_points(ctx: &egui::Context) -> f32 {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send, sel, Encode, Encoding};

    if !ctx.egui_wants_keyboard_input() {
        return 0.0;
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }
    unsafe impl Encode for CGPoint {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGSize {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }
    unsafe impl Encode for CGRect {
        const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
    }

    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if !app.is_null() {
            let windows: *mut AnyObject = msg_send![app, windows];
            if !windows.is_null() {
                let count: usize = msg_send![windows, count];
                for idx in 0..count {
                    let win: *mut AnyObject = msg_send![windows, objectAtIndex: idx];
                    if win.is_null() {
                        continue;
                    }
                    let has_guide: bool = msg_send![win, respondsToSelector: sel!(keyboardLayoutGuide)];
                    if has_guide {
                        let guide: *mut AnyObject = msg_send![win, keyboardLayoutGuide];
                        if !guide.is_null() {
                            let frame: CGRect = msg_send![guide, layoutFrame];
                            if frame.size.height > 80.0 {
                                return frame.size.height as f32;
                            }
                        }
                    }
                }
            }
        }
    }

    let screen = ctx.content_rect();
    if screen.height() >= screen.width() {
        // iPhone portrait soft keyboard height (including predictions bar)
        336.0
    } else {
        // iPhone landscape soft keyboard height
        216.0
    }
}

#[cfg(target_os = "android")]
fn android_open_url(url: &str) {
    let ctx = ndk_context::android_context();
    if ctx.vm().is_null() || ctx.context().is_null() {
        return;
    }
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) };
    let activity_ptr: jni::sys::jobject = ctx.context().cast();
    let _ = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = unsafe { jni::objects::JObject::from_raw(env, activity_ptr) };
        let j_url = env.new_string(url)?;
        let j_action = env.new_string("android.intent.action.VIEW")?;
        let uri = env
            .call_static_method(
                jni::jni_str!("android/net/Uri"),
                jni::jni_str!("parse"),
                jni::jni_sig!("(Ljava/lang/String;)Landroid/net/Uri;"),
                &[jni::objects::JValue::Object(&j_url)],
            )?
            .l()?;
        let intent = env.new_object(
            jni::jni_str!("android/content/Intent"),
            jni::jni_sig!("(Ljava/lang/String;Landroid/net/Uri;)V"),
            &[
                jni::objects::JValue::Object(&j_action),
                jni::objects::JValue::Object(&uri),
            ],
        )?;
        env.call_method(
            &activity,
            jni::jni_str!("startActivity"),
            jni::jni_sig!("(Landroid/content/Intent;)V"),
            &[jni::objects::JValue::Object(&intent)],
        )?;
        std::mem::forget(activity);
        Ok(())
    });
}

// ── Soft Keyboard Dismissal (Tap Outside TextEdit) ──────────────────────────

/// Dismisses the OS soft keyboard on iOS (`UIWindow endEditing:YES`) and Android
/// whenever the user taps outside a focused `egui::TextEdit`.
pub fn dismiss_soft_keyboard() {
    #[cfg(target_os = "ios")]
    {
        ios_dismiss_keyboard();
    }
    #[cfg(target_os = "android")]
    {
        crate::android_ime::reset_ime_composing();
    }
}

#[cfg(target_os = "ios")]
fn ios_dismiss_keyboard() {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let _: bool = msg_send![
            app,
            sendAction: objc2::sel!(resignFirstResponder),
            to: std::ptr::null::<AnyObject>(),
            from: std::ptr::null::<AnyObject>(),
            forEvent: std::ptr::null::<AnyObject>()
        ];
        let windows: *mut AnyObject = msg_send![app, windows];
        if !windows.is_null() {
            let count: usize = msg_send![windows, count];
            for idx in 0..count {
                let win: *mut AnyObject = msg_send![windows, objectAtIndex: idx];
                if !win.is_null() {
                    let _: bool = msg_send![win, endEditing: true];
                }
            }
        }
    }
}

// ── iOS Memory Warning Observer ─────────────────────────────────────────────

static MEMORY_WARNING_PENDING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Detects Adreno (600/700+) or Mali (Valhall/Bifrost) GPUs on Android before
/// `wgpu::Instance` creation and loads a bundled Turnip (`libvulkan_freedreno.so`)
/// or PanVK (`libvulkan_panfrost.so`) ICD from `nativeLibraryDir` if present.
/// Cleanly falls back to the system Vulkan/GLES driver on Waydroid or stock devices.
pub fn init_android_vulkan_driver() {
    #[cfg(target_os = "android")]
    {
        if std::env::var_os("VK_ICD_FILENAMES").is_some() || std::env::var_os("VK_DRIVER_FILES").is_some() {
            return;
        }
        // Discover nativeLibraryDir from our own loaded shared object in /proc/self/maps
        let native_lib_dir = std::fs::read_to_string("/proc/self/maps")
            .ok()
            .and_then(|maps| {
                maps.lines().find_map(|line| {
                    let path = line.split_whitespace().last()?;
                    if path.ends_with("librtrust_mobile.so") {
                        std::path::Path::new(path).parent().map(|p| p.to_path_buf())
                    } else {
                        None
                    }
                })
            });
        let Some(lib_dir) = native_lib_dir else { return };

        // Detect Adreno 600/700+ via KGSL sysfs or Mali via /sys/class/misc/mali0
        let adreno_model = std::fs::read_to_string("/sys/class/kgsl/kgsl-3d0/gpu_model")
            .ok()
            .map(|s| s.trim().to_string());
        let is_adreno_modern = adreno_model.as_deref().is_some_and(|m| {
            let lower = m.to_ascii_lowercase();
            lower.contains("adreno")
                && (lower.contains("6") || lower.contains("7") || lower.contains("8"))
        });
        let is_mali = std::path::Path::new("/sys/class/misc/mali0").exists();

        let custom_so = if is_adreno_modern {
            let p = lib_dir.join("libvulkan_freedreno.so");
            p.exists().then_some(("turnip_icd.json", p))
        } else if is_mali {
            let p = lib_dir.join("libvulkan_panfrost.so");
            p.exists().then_some(("panvk_icd.json", p))
        } else {
            None
        };

        if let Some((icd_name, so_path)) = custom_so {
            let cache_dir = std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let _ = std::fs::create_dir_all(&cache_dir);
            let icd_path = cache_dir.join(icd_name);
            let manifest = format!(
                "{{\"file_format_version\":\"1.0.0\",\"ICD\":{{\"library_path\":\"{}\",\"api_version\":\"1.3.0\"}}}}",
                so_path.display()
            );
            if std::fs::write(&icd_path, manifest).is_ok() {
                unsafe {
                    std::env::set_var("VK_ICD_FILENAMES", &icd_path);
                    std::env::set_var("VK_DRIVER_FILES", &icd_path);
                }
                log::info!("RtRustMobile: loaded custom Android Vulkan ICD {}", icd_path.display());
            }
        } else {
            log::info!(
                "RtRustMobile: using system Vulkan/GLES driver (adreno={:?}, mali={})",
                adreno_model,
                is_mali
            );
        }
    }
}

/// Queries total physical device RAM in bytes (NSProcessInfo on iOS, /proc/meminfo on Android/Linux).
pub fn physical_memory_bytes() -> u64 {
    #[cfg(target_os = "ios")]
    {
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};
        unsafe {
            let info: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
            if !info.is_null() {
                let mem: u64 = msg_send![info, physicalMemory];
                if mem > 0 {
                    return mem;
                }
            }
        }
    }
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    if let Some(kb) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    4 * 1024 * 1024 * 1024
}

/// Queries current process resident memory (RSS) in bytes on Android/Linux.
pub fn process_resident_bytes() -> u64 {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        if let Ok(text) = std::fs::read_to_string("/proc/self/status") {
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    if let Some(kb) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    0
}

/// Registers an `UIApplicationDidReceiveMemoryWarningNotification` observer on iOS
/// so the main loop can immediately purge unused asset caches and run Luau GC.
pub fn install_memory_warning_observer() {
    static INIT_RAM: std::sync::Once = std::sync::Once::new();
    INIT_RAM.call_once(|| {
        rustudio_core::mobile::set_device_ram_bytes(physical_memory_bytes());
    });
    #[cfg(target_os = "ios")]
    {
        use objc2::declare::ClassBuilder;
        use objc2::runtime::{AnyClass, AnyObject, Sel};
        use objc2::{class, msg_send, sel};
        use objc2_foundation::NSString;
        use std::sync::Once;

        static REGISTER_OBSERVER: Once = Once::new();
        REGISTER_OBSERVER.call_once(|| {
            unsafe extern "C" fn on_memory_warning(
                _this: *mut AnyObject,
                _cmd: Sel,
                _note: *mut AnyObject,
            ) {
                MEMORY_WARNING_PENDING.store(true, std::sync::atomic::Ordering::SeqCst);
            }

            unsafe {
                let cls: *const AnyClass = if let Some(existing) = AnyClass::get(c"RtRustMemoryWarningObserver") {
                    existing as *const AnyClass
                } else {
                    let superclass = class!(NSObject);
                    let mut builder = ClassBuilder::new(c"RtRustMemoryWarningObserver", superclass)
                        .expect("RtRustMemoryWarningObserver class builder");
                    builder.add_method(
                        sel!(onMemoryWarning:),
                        on_memory_warning as unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
                    );
                    builder.register() as *const AnyClass
                };
                let observer: *mut AnyObject = msg_send![cls, alloc];
                let observer: *mut AnyObject = msg_send![observer, init];
                let center: *mut AnyObject = msg_send![class!(NSNotificationCenter), defaultCenter];
                let name = NSString::from_str("UIApplicationDidReceiveMemoryWarningNotification");
                let _: () = msg_send![
                    center,
                    addObserver: observer,
                    selector: sel!(onMemoryWarning:),
                    name: &*name,
                    object: std::ptr::null::<AnyObject>()
                ];
            }
        });
    }
}

/// Returns `true` once if iOS emitted `UIApplicationDidReceiveMemoryWarningNotification`.
pub fn take_memory_warning() -> bool {
    MEMORY_WARNING_PENDING.swap(false, std::sync::atomic::Ordering::SeqCst)
}

// ── Cross-Platform Wallpaper Image Picker (iOS / Android / Desktop) ─────────

static PENDING_WALLPAPER_PICK: std::sync::Mutex<Option<(String, PathBuf)>> =
    std::sync::Mutex::new(None);
static ACTIVE_PICK_TARGET: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Drains any newly picked wallpaper image from the native iOS/Android/Desktop picker
/// and notifies `crate::wallpaper::notify_picked_wallpaper`.
pub fn poll_picked_wallpaper() {
    if let Ok(mut guard) = PENDING_WALLPAPER_PICK.lock() {
        if let Some((target, path)) = guard.take() {
            drop(guard);
            crate::wallpaper::notify_picked_wallpaper(&target, path);
        }
    }
}

/// Opens the native device image picker for `"studio"` or `"code"` wallpaper target.
pub fn pick_wallpaper_image(target: &str) {
    if let Ok(mut g) = ACTIVE_PICK_TARGET.lock() {
        *g = Some(target.to_string());
    }
    #[cfg(target_os = "ios")]
    {
        ios_pick_wallpaper_image(target);
        return;
    }
    #[cfg(target_os = "android")]
    {
        android_pick_wallpaper_image(target);
        return;
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        let target_owned = target.to_string();
        std::thread::spawn(move || {
            if let Some(src_path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
            {
                let dir = app_data_dir().join("wallpapers");
                let _ = std::fs::create_dir_all(&dir);
                let ext = src_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("png");
                let dest = dir.join(format!("custom_{target_owned}.{ext}"));
                if std::fs::copy(&src_path, &dest).is_ok() {
                    if let Ok(mut g) = PENDING_WALLPAPER_PICK.lock() {
                        *g = Some((target_owned, dest));
                    }
                }
            }
        });
    }
}

#[cfg(target_os = "ios")]
fn ios_pick_wallpaper_image(_target: &str) {
    use objc2::declare::ClassBuilder;
    use objc2::runtime::{AnyClass, AnyObject, Sel};
    use objc2::{class, msg_send, sel};
    use objc2_foundation::NSString;
    use std::sync::Once;

    static REGISTER_DELEGATE: Once = Once::new();

    unsafe extern "C" fn did_finish_picking(
        _this: *mut AnyObject,
        _cmd: Sel,
        picker: *mut AnyObject,
        info: *mut AnyObject,
    ) {
        unsafe {
            if !info.is_null() {
                let target_str = ACTIVE_PICK_TARGET
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_else(|| "studio".to_string());
                let dir = app_data_dir().join("wallpapers");
                let _ = std::fs::create_dir_all(&dir);
                let dest = dir.join(format!("ios_{target_str}_wallpaper.png"));

                // 1. Check UIImagePickerControllerImageURL first (preserves original PNG/JPEG bytes)
                let key_url = NSString::from_str("UIImagePickerControllerImageURL");
                let ns_url: *mut AnyObject = msg_send![info, objectForKey: &*key_url];
                let mut copied = false;
                if !ns_url.is_null() {
                    let path_ns: *mut AnyObject = msg_send![ns_url, path];
                    if !path_ns.is_null() {
                        let utf8: *const std::ffi::c_char = msg_send![path_ns, UTF8String];
                        if !utf8.is_null() {
                            let s = std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned();
                            if std::fs::copy(&s, &dest).is_ok() {
                                copied = true;
                            }
                        }
                    }
                }

                // 2. Fallback: write UIImagePickerControllerOriginalImage via NSData writeToFile
                if !copied {
                    let key_img = NSString::from_str("UIImagePickerControllerOriginalImage");
                    let ui_img: *mut AnyObject = msg_send![info, objectForKey: &*key_img];
                    if !ui_img.is_null() {
                        extern "C" {
                            fn UIImagePNGRepresentation(image: *mut AnyObject) -> *mut AnyObject;
                        }
                        let png_data = UIImagePNGRepresentation(ui_img);
                        if !png_data.is_null() {
                            let dest_str = NSString::from_str(&dest.display().to_string());
                            let ok: bool = msg_send![png_data, writeToFile: &*dest_str, atomically: true];
                            if ok {
                                copied = true;
                            }
                        }
                    }
                }

                if copied {
                    if let Ok(mut g) = PENDING_WALLPAPER_PICK.lock() {
                        *g = Some((target_str, dest));
                    }
                }
            }
            if !picker.is_null() {
                let null_block: *const AnyObject = std::ptr::null();
                let _: () = msg_send![picker, dismissViewControllerAnimated: true, completion: null_block];
            }
        }
    }

    unsafe extern "C" fn did_cancel(
        _this: *mut AnyObject,
        _cmd: Sel,
        picker: *mut AnyObject,
    ) {
        unsafe {
            if !picker.is_null() {
                let null_block: *const AnyObject = std::ptr::null();
                let _: () = msg_send![picker, dismissViewControllerAnimated: true, completion: null_block];
            }
        }
    }

    REGISTER_DELEGATE.call_once(|| unsafe {
        if let Some(mut builder) = ClassBuilder::new(c"RtRustWallpaperPickerDelegate", class!(NSObject)) {
            builder.add_method(
                sel!(imagePickerController:didFinishPickingMediaWithInfo:),
                did_finish_picking
                    as unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
            );
            builder.add_method(
                sel!(imagePickerControllerDidCancel:),
                did_cancel as unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            let _ = builder.register();
        }
    });

    unsafe {
        let Some(del_cls) = AnyClass::get(c"RtRustWallpaperPickerDelegate") else {
            return;
        };
        let delegate: *mut AnyObject = msg_send![del_cls, new];
        let picker: *mut AnyObject = msg_send![class!(UIImagePickerController), new];
        if picker.is_null() || delegate.is_null() {
            return;
        }
        // sourceType 0 = UIImagePickerControllerSourceTypePhotoLibrary
        let _: () = msg_send![picker, setSourceType: 0isize];
        let _: () = msg_send![picker, setDelegate: delegate];

        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let windows: *mut AnyObject = msg_send![app, windows];
        if windows.is_null() {
            return;
        }
        let count: usize = msg_send![windows, count];
        for idx in 0..count {
            let win: *mut AnyObject = msg_send![windows, objectAtIndex: idx];
            if win.is_null() {
                continue;
            }
            let root_vc: *mut AnyObject = msg_send![win, rootViewController];
            if !root_vc.is_null() {
                let null_block: *const AnyObject = std::ptr::null();
                let _: () = msg_send![
                    root_vc,
                    presentViewController: picker,
                    animated: true,
                    completion: null_block
                ];
                break;
            }
        }
    }
}

#[cfg(target_os = "android")]
fn android_pick_wallpaper_image(target: &str) {
    let dir = app_data_dir().join("wallpapers");
    let _ = std::fs::create_dir_all(&dir);
    for candidate in ["/sdcard/Download/wallpaper.png", "/sdcard/Download/wallpaper.jpg", "/sdcard/Pictures/wallpaper.png"] {
        let p = Path::new(candidate);
        if p.exists() {
            let dest = dir.join(format!("android_{target}.png"));
            if std::fs::copy(p, &dest).is_ok() {
                if let Ok(mut g) = PENDING_WALLPAPER_PICK.lock() {
                    *g = Some((target.to_string(), dest));
                }
                return;
            }
        }
    }
    let ctx = ndk_context::android_context();
    if ctx.vm().is_null() || ctx.context().is_null() {
        return;
    }
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) };
    let activity_ptr: jni::sys::jobject = ctx.context().cast();
    let _ = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = unsafe { jni::objects::JObject::from_raw(env, activity_ptr) };
        let j_action = env.new_string("android.intent.action.OPEN_DOCUMENT")?;
        let j_type = env.new_string("image/*")?;
        let intent = env.new_object(
            jni::jni_str!("android/content/Intent"),
            jni::jni_sig!("(Ljava/lang/String;)V"),
            &[jni::objects::JValue::Object(&j_action)],
        )?;
        let _ = env.call_method(
            &intent,
            jni::jni_str!("setType"),
            jni::jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
            &[jni::objects::JValue::Object(&j_type)],
        )?;
        let _ = env.call_method(
            &activity,
            jni::jni_str!("startActivity"),
            jni::jni_sig!("(Landroid/content/Intent;)V"),
            &[jni::objects::JValue::Object(&intent)],
        );
        std::mem::forget(activity);
        Ok(())
    });
}

