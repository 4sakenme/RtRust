//! Mobile Eco Guard & touch-layout runtime flags shared across rustudio crates.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::time::Duration;

static TOUCH_KEYS: AtomicU8 = AtomicU8::new(0);
static TOUCH_STICK: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn set_touch_stick(x: f32, y: f32) {
    let packed = ((x.to_bits() as u64) << 32) | (y.to_bits() as u64);
    TOUCH_STICK.store(packed, Ordering::Relaxed);
}

#[inline]
pub fn touch_stick() -> (f32, f32) {
    let packed = TOUCH_STICK.load(Ordering::Relaxed);
    let x = f32::from_bits((packed >> 32) as u32);
    let y = f32::from_bits(packed as u32);
    if x.is_finite() && y.is_finite() {
        (x, y)
    } else {
        (0.0, 0.0)
    }
}

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
/// Enabled by default on Android & iOS; toggleable from the top toolbar on any platform.
pub static MOBILE_ECO_MODE: AtomicBool = AtomicBool::new(cfg!(any(target_os = "android", target_os = "ios")));

/// Whether touch-friendly floating drawers & virtual controls are active.
/// Enabled by default on Android & iOS, and auto-activated on narrow screens or via toggle.
pub static MOBILE_DRAWER_MODE: AtomicBool = AtomicBool::new(cfg!(any(target_os = "android", target_os = "ios")));

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
pub static SHIFT_LOCK: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn set_touch_cam_held(held: bool) {
    TOUCH_CAM_HELD.store(held, Ordering::Relaxed);
}

#[inline]
pub fn touch_cam_held() -> bool {
    TOUCH_CAM_HELD.load(Ordering::Relaxed)
}

#[inline]
pub fn shift_lock() -> bool {
    SHIFT_LOCK.load(Ordering::Relaxed)
}

#[inline]
pub fn set_shift_lock(enabled: bool) {
    SHIFT_LOCK.store(enabled, Ordering::Relaxed);
}

#[inline]
pub fn toggle_shift_lock() -> bool {
    let next = !shift_lock();
    set_shift_lock(next);
    next
}

static PLAY_CAM_DX: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static PLAY_CAM_DY: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static PLAY_CAM_DZOOM: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn atomic_add_f32(cell: &std::sync::atomic::AtomicU32, val: f32) {
    let mut cur = cell.load(Ordering::Relaxed);
    loop {
        let next = (f32::from_bits(cur) + val).to_bits();
        match cell.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => cur = actual,
        }
    }
}

/// Accumulates camera orbit `(dx, dy)` and zoom `dzoom` directly from the UI
/// viewport (`src/viewport.rs`) so Playtest camera rotation works reliably
/// even across background play worker threads.
pub fn add_play_cam_delta(dx: f32, dy: f32, dzoom: f32) {
    if dx != 0.0 {
        atomic_add_f32(&PLAY_CAM_DX, dx);
    }
    if dy != 0.0 {
        atomic_add_f32(&PLAY_CAM_DY, dy);
    }
    if dzoom != 0.0 {
        atomic_add_f32(&PLAY_CAM_DZOOM, dzoom);
    }
}

/// Whether Playtest Eco profile is currently active (mobile Eco + Playtest running).
/// Directly overrides engine lighting/shadows/post-processing to prevent mobile OOM & lag.
pub static PLAYTEST_ECO_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether Portrait mode auto-scales in-game ScreenGuis (`PlayerGui`/`StarterGui`)
/// to a 16:9 landscape reference container so UIs never squish vertically.
pub static GUI_AUTO_LANDSCAPE: AtomicBool = AtomicBool::new(true);

/// Total physical device RAM in bytes (queried at startup via NSProcessInfo / /proc/meminfo).
static DEVICE_RAM_BYTES: AtomicU64 = AtomicU64::new(4 * 1024 * 1024 * 1024);

#[inline]
pub fn is_playtest_eco_active() -> bool {
    PLAYTEST_ECO_ACTIVE.load(Ordering::Relaxed)
}

#[inline]
pub fn set_playtest_eco_active(active: bool) {
    PLAYTEST_ECO_ACTIVE.store(active, Ordering::Relaxed);
}

#[inline]
pub fn gui_auto_landscape() -> bool {
    GUI_AUTO_LANDSCAPE.load(Ordering::Relaxed)
}

#[inline]
pub fn toggle_gui_auto_landscape() -> bool {
    let next = !gui_auto_landscape();
    GUI_AUTO_LANDSCAPE.store(next, Ordering::Relaxed);
    next
}

#[inline]
pub fn set_device_ram_bytes(bytes: u64) {
    if bytes > 0 {
        DEVICE_RAM_BYTES.store(bytes, Ordering::Relaxed);
    }
}

/// Hard Playtest memory ceiling: 50% of physical device RAM, clamped safely below
/// iOS's 2098 MB Jetsam limit (e.g., ~1024 MB on 4 GB devices like iPhone 11 Pro).
#[inline]
pub fn playtest_ram_budget_bytes() -> u64 {
    let total = DEVICE_RAM_BYTES.load(Ordering::Relaxed);
    let half = total / 2;
    if total <= 4 * 1024 * 1024 * 1024 {
        half.clamp(512 * 1024 * 1024, 1024 * 1024 * 1024)
    } else {
        half.clamp(1024 * 1024 * 1024, 1536 * 1024 * 1024)
    }
}

/// Drains accumulated `(dx, dy, dzoom)` for the current Playtest step.
pub fn take_play_cam_delta() -> (f32, f32, f32) {
    let dx = f32::from_bits(PLAY_CAM_DX.swap(0, Ordering::Relaxed));
    let dy = f32::from_bits(PLAY_CAM_DY.swap(0, Ordering::Relaxed));
    let dz = f32::from_bits(PLAY_CAM_DZOOM.swap(0, Ordering::Relaxed));
    (dx, dy, dz)
}
