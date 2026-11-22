//! Mobile Eco Guard & touch-layout runtime flags shared across rustudio crates.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

static TOUCH_KEYS: AtomicU8 = AtomicU8::new(0);

#[inline]
pub fn set_touch_keys(keys: [bool; 5]) {
    let mask = (keys[0] as u8)
        | ((keys[1] as u8) << 1)
        | ((keys[2] as u8) << 2)
        | ((keys[3] as u8) << 3)
        | ((keys[4] as u8) << 4);
    TOUCH_KEYS.store(mask, Ordering::Relaxed);
}

#[inline]
pub fn touch_keys() -> [bool; 5] {
    let m = TOUCH_KEYS.load(Ordering::Relaxed);
    [
        (m & 1) != 0,
        (m & 2) != 0,
        (m & 4) != 0,
        (m & 8) != 0,
        (m & 16) != 0,
    ]
}

/// Eco mode: No Shadows, 0.7x viewport render scale, 30 FPS cap during Play,
/// and On-Demand rendering (`ControlFlow::Wait`) when idle.
/// Enabled by default on Android; toggleable from the top toolbar on any platform.
pub static MOBILE_ECO_MODE: AtomicBool = AtomicBool::new(cfg!(target_os = "android"));

/// Whether touch-friendly floating drawers & virtual controls are active.
/// Enabled by default on Android, and auto-activated on narrow screens or via toggle.
pub static MOBILE_DRAWER_MODE: AtomicBool = AtomicBool::new(cfg!(target_os = "android"));

#[inline]
pub fn is_eco() -> bool {
    MOBILE_ECO_MODE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_eco(enabled: bool) {
    MOBILE_ECO_MODE.store(enabled, Ordering::Relaxed);
}

#[inline]
pub fn toggle_eco() -> bool {
    let next = !is_eco();
    set_eco(next);
    next
}

#[inline]
pub fn is_drawer_mode() -> bool {
    MOBILE_DRAWER_MODE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_drawer_mode(enabled: bool) {
    MOBILE_DRAWER_MODE.store(enabled, Ordering::Relaxed);
}

#[inline]
pub fn render_scale() -> f32 {
    if is_eco() { 0.7 } else { 1.0 }
}

#[inline]
pub fn play_frame_interval() -> Duration {
    if is_eco() {
        Duration::from_millis(33) // ~30 FPS Eco cap
    } else {
        Duration::from_millis(16) // ~60 FPS High
    }
}

pub static TOUCH_CAM_HELD: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn set_touch_cam_held(held: bool) {
    TOUCH_CAM_HELD.store(held, Ordering::Relaxed);
}

#[inline]
pub fn touch_cam_held() -> bool {
    TOUCH_CAM_HELD.load(Ordering::Relaxed)
}
