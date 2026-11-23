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
    if !ctx.egui_wants_keyboard_input() {
        return 0.0;
    }
    let screen = ctx.content_rect();
    if screen.height() >= screen.width() {
        // iPhone portrait soft keyboard height (including predictions bar)
        302.0
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
