//! RTRUST DXGI Shim (`dxgi.dll`)
//!
//! Intermediary graphics & resource proxy placed next to `RobloxStudioBeta.exe`.
//! Forwards all DirectX/Vulkan presentation calls to `dxgi_vk.dll` (DXVK) while
//! enforcing active GPU resource throttling, alt-tab power saving, and VRAM limits.

#![allow(non_snake_case)]
#![allow(static_mut_refs)]
#![allow(clippy::all)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Once;

pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;
pub const E_FAIL: HRESULT = -2147467259;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

#[repr(C)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub struct DXGI_ADAPTER_DESC {
    pub Description: [u16; 128],
    pub VendorId: u32,
    pub DeviceId: u32,
    pub SubSysId: u32,
    pub Revision: u32,
    pub DedicatedVideoMemory: usize,
    pub DedicatedSystemMemory: usize,
    pub SharedSystemMemory: usize,
    pub AdapterLuid: [u32; 2],
}

#[repr(C)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct DXGI_ADAPTER_DESC1 {
    pub Description: [u16; 128],
    pub VendorId: u32,
    pub DeviceId: u32,
    pub SubSysId: u32,
    pub Revision: u32,
    pub DedicatedVideoMemory: usize,
    pub DedicatedSystemMemory: usize,
    pub SharedSystemMemory: usize,
    pub AdapterLuid: [u32; 2],
    pub Flags: u32,
}

// Win32 External Declarations
extern "system" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *mut c_void;
    fn GetModuleHandleA(lpModuleName: *const u8) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(hWnd: *mut c_void, lpdwProcessId: *mut u32) -> u32;
    fn GetCurrentProcessId() -> u32;
    fn Sleep(dwMilliseconds: u32);
    fn VirtualProtect(
        lpAddress: *mut c_void,
        dwSize: usize,
        flNewProtect: u32,
        lpflOldProtect: *mut u32,
    ) -> i32;
    fn GetTickCount64() -> u64;
    fn GetAsyncKeyState(vKey: i32) -> i16;
    fn GetWindowRect(hWnd: *mut c_void, lpRect: *mut RECT) -> i32;
    fn ClipCursor(lpRect: *const RECT) -> i32;
    fn ShowCursor(bShow: i32) -> i32;
    fn SetCursorPos(X: i32, Y: i32) -> i32;
}

// Function pointer signatures for DXGI exports
type FnCreateDXGIFactory = unsafe extern "system" fn(*const GUID, *mut *mut c_void) -> HRESULT;
type FnCreateDXGIFactory1 = unsafe extern "system" fn(*const GUID, *mut *mut c_void) -> HRESULT;
type FnCreateDXGIFactory2 = unsafe extern "system" fn(u32, *const GUID, *mut *mut c_void) -> HRESULT;
type FnDXGIDeclareAdapterRemovalSupport = unsafe extern "system" fn() -> HRESULT;
type FnDXGIGetDebugInterface1 = unsafe extern "system" fn(u32, *const GUID, *mut *mut c_void) -> HRESULT;
type FnDXGID3D10CreateDevice = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut c_void,
    u32,
    *const c_void,
    u32,
    *mut *mut c_void,
) -> HRESULT;
type FnDXGID3D10RegisterLayers = unsafe extern "system" fn(*const c_void, u32) -> HRESULT;

// Function pointer signatures for Hooked VMT methods
type FnPresent = unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;
type FnPresent1 = unsafe extern "system" fn(*mut c_void, u32, u32, *const c_void) -> HRESULT;
type FnCreateSwapChain = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT;
type FnCreateSwapChainForHwnd = unsafe extern "system" fn(
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *const c_void,
    *const c_void,
    *const c_void,
    *mut *mut c_void,
) -> HRESULT;
type FnEnumAdapters = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT;
type FnEnumAdapters1 = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> HRESULT;
type FnGetDesc = unsafe extern "system" fn(*mut c_void, *mut DXGI_ADAPTER_DESC) -> HRESULT;
type FnGetDesc1 = unsafe extern "system" fn(*mut c_void, *mut DXGI_ADAPTER_DESC1) -> HRESULT;

static INIT_ONCE: Once = Once::new();
static mut REAL_DXGI_MODULE: *mut c_void = std::ptr::null_mut();

// Original functions from real DXGI
static mut REAL_CREATE_FACTORY: Option<FnCreateDXGIFactory> = None;
static mut REAL_CREATE_FACTORY1: Option<FnCreateDXGIFactory1> = None;
static mut REAL_CREATE_FACTORY2: Option<FnCreateDXGIFactory2> = None;
static mut REAL_DECLARE_ADAPTER_REMOVAL: Option<FnDXGIDeclareAdapterRemovalSupport> = None;
static mut REAL_GET_DEBUG_INTERFACE1: Option<FnDXGIGetDebugInterface1> = None;
static mut REAL_D3D10_CREATE_DEVICE: Option<FnDXGID3D10CreateDevice> = None;
static mut REAL_D3D10_REGISTER_LAYERS: Option<FnDXGID3D10RegisterLayers> = None;

// Original hooked methods
static mut REAL_PRESENT: Option<FnPresent> = None;
static mut REAL_PRESENT1: Option<FnPresent1> = None;
static mut REAL_CREATE_SWAP_CHAIN: Option<FnCreateSwapChain> = None;
static mut REAL_CREATE_SWAP_CHAIN_HWND: Option<FnCreateSwapChainForHwnd> = None;
static mut REAL_ENUM_ADAPTERS: Option<FnEnumAdapters> = None;
static mut REAL_ENUM_ADAPTERS1: Option<FnEnumAdapters1> = None;
static mut REAL_GET_DESC: Option<FnGetDesc> = None;
static mut REAL_GET_DESC1: Option<FnGetDesc1> = None;

// Configuration options
static ALT_TAB_THROTTLE: AtomicBool = AtomicBool::new(true);
static ALT_TAB_SLEEP_MS: AtomicU32 = AtomicU32::new(100); // 100ms sleep ~ 10 FPS in background
static VRAM_CAP_MB: AtomicU32 = AtomicU32::new(1024);     // Report 1024 MB to clamp texture streaming

// Telemetry counters
static FRAME_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_TELEMETRY_TIME: AtomicU64 = AtomicU64::new(0);
static CURRENT_FPS: AtomicU32 = AtomicU32::new(0);

fn init_real_dxgi() {
    INIT_ONCE.call_once(|| unsafe {
        load_shim_configuration();
        install_mouse_iat_hooks();

        // 1. Try to load dxgi_vk.dll (DXVK's renamed dxgi.dll)
        let mut hmod = LoadLibraryA(b"dxgi_vk.dll\0".as_ptr());
        if hmod.is_null() {
            // Fallback: try wine system dxgi
            hmod = LoadLibraryA(b"C:\\windows\\system32\\dxgi.dll\0".as_ptr());
        }
        if hmod.is_null() {
            hmod = LoadLibraryA(b"dxgi.dll\0".as_ptr());
        }

        REAL_DXGI_MODULE = hmod;
        if !hmod.is_null() {
            let p_cf = GetProcAddress(hmod, b"CreateDXGIFactory\0".as_ptr());
            if !p_cf.is_null() {
                REAL_CREATE_FACTORY = Some(std::mem::transmute(p_cf));
            }
            let p_cf1 = GetProcAddress(hmod, b"CreateDXGIFactory1\0".as_ptr());
            if !p_cf1.is_null() {
                REAL_CREATE_FACTORY1 = Some(std::mem::transmute(p_cf1));
            }
            let p_cf2 = GetProcAddress(hmod, b"CreateDXGIFactory2\0".as_ptr());
            if !p_cf2.is_null() {
                REAL_CREATE_FACTORY2 = Some(std::mem::transmute(p_cf2));
            }
            let p_dar = GetProcAddress(hmod, b"DXGIDeclareAdapterRemovalSupport\0".as_ptr());
            if !p_dar.is_null() {
                REAL_DECLARE_ADAPTER_REMOVAL = Some(std::mem::transmute(p_dar));
            }
            let p_cdev = GetProcAddress(hmod, b"DXGID3D10CreateDevice\0".as_ptr());
            if !p_cdev.is_null() {
                REAL_D3D10_CREATE_DEVICE = Some(std::mem::transmute(p_cdev));
            }
            let p_rl = GetProcAddress(hmod, b"DXGID3D10RegisterLayers\0".as_ptr());
            if !p_rl.is_null() {
                REAL_D3D10_REGISTER_LAYERS = Some(std::mem::transmute(p_rl));
            }
            let p_dbg = GetProcAddress(hmod, b"DXGIGetDebugInterface1\0".as_ptr());
            if !p_dbg.is_null() {
                REAL_GET_DEBUG_INTERFACE1 = Some(std::mem::transmute(p_dbg));
            }
        }
    });
}

fn load_shim_configuration() {
    // Check config file first
    if let Ok(content) = std::fs::read_to_string("C:\\rtrust_shim.json") {
        parse_config_json(&content);
        return;
    }
    if let Ok(content) = std::fs::read_to_string("rtrust_shim.json") {
        parse_config_json(&content);
        return;
    }

    // Check environment variables
    if let Ok(val) = std::env::var("RTRUST_ALT_TAB_THROTTLE") {
        ALT_TAB_THROTTLE.store(val == "1" || val.to_lowercase() == "true", Ordering::Relaxed);
    }
    if let Ok(val) = std::env::var("RTRUST_ALT_TAB_SLEEP_MS") {
        if let Ok(ms) = val.parse::<u32>() {
            ALT_TAB_SLEEP_MS.store(ms, Ordering::Relaxed);
        }
    }
    if let Ok(val) = std::env::var("RTRUST_VRAM_CAP_MB") {
        if let Ok(mb) = val.parse::<u32>() {
            VRAM_CAP_MB.store(mb, Ordering::Relaxed);
        }
    }
}

fn parse_config_json(json_str: &str) {
    if json_str.contains("\"alt_tab_throttle\": false") {
        ALT_TAB_THROTTLE.store(false, Ordering::Relaxed);
    } else {
        ALT_TAB_THROTTLE.store(true, Ordering::Relaxed);
    }

    if let Some(pos) = json_str.find("\"alt_tab_sleep_ms\":") {
        let rest = &json_str[pos + 19..];
        if let Some(end) = rest.find(|c: char| c == ',' || c == '}') {
            if let Ok(ms) = rest[..end].trim().parse::<u32>() {
                ALT_TAB_SLEEP_MS.store(ms, Ordering::Relaxed);
            }
        }
    }

    if let Some(pos) = json_str.find("\"vram_cap_mb\":") {
        let rest = &json_str[pos + 14..];
        if let Some(end) = rest.find(|c: char| c == ',' || c == '}') {
            if let Ok(mb) = rest[..end].trim().parse::<u32>() {
                VRAM_CAP_MB.store(mb, Ordering::Relaxed);
            }
        }
    }
}

unsafe fn hook_vtable_entry(vtable_ptr: *mut usize, method_index: usize, hook_fn: usize) -> usize {
    let target = vtable_ptr.add(method_index);
    let mut old_protect = 0;
    if VirtualProtect(target as *mut c_void, std::mem::size_of::<usize>(), PAGE_EXECUTE_READWRITE, &mut old_protect) != 0 {
        let orig = *target;
        if orig != hook_fn {
            *target = hook_fn;
        }
        VirtualProtect(target as *mut c_void, std::mem::size_of::<usize>(), old_protect, &mut old_protect);
        orig
    } else {
        *target
    }
}

type FnSetCursorPos = unsafe extern "system" fn(i32, i32) -> i32;
static mut REAL_SET_CURSOR_POS: Option<FnSetCursorPos> = None;
static ACTIVE_RMB_CONFINED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct IMAGE_DOS_HEADER {
    e_magic: u16,
    _pad: [u8; 58],
    e_lfanew: i32,
}

#[repr(C)]
struct IMAGE_DATA_DIRECTORY {
    virtual_address: u32,
    size: u32,
}

#[repr(C)]
struct IMAGE_IMPORT_DESCRIPTOR {
    original_first_thunk: u32,
    time_date_stamp: u32,
    forwarder_chain: u32,
    name: u32,
    first_thunk: u32,
}

#[no_mangle]
pub unsafe extern "system" fn Hooked_SetCursorPos(x: i32, y: i32) -> i32 {
    // Hide cursor temporarily so Xwayland permits XWarpPointer without dropping the delta
    ShowCursor(0);
    let res = if let Some(orig) = REAL_SET_CURSOR_POS {
        orig(x, y)
    } else {
        SetCursorPos(x, y)
    };
    ShowCursor(1);
    res
}

unsafe fn handle_mouse_confinement() {
    let fg_hwnd = GetForegroundWindow();
    if fg_hwnd.is_null() {
        if ACTIVE_RMB_CONFINED.swap(false, Ordering::Relaxed) {
            ClipCursor(std::ptr::null());
        }
        return;
    }

    let mut fg_pid = 0;
    GetWindowThreadProcessId(fg_hwnd, &mut fg_pid);
    if fg_pid != GetCurrentProcessId() {
        if ACTIVE_RMB_CONFINED.swap(false, Ordering::Relaxed) {
            ClipCursor(std::ptr::null());
        }
        return;
    }

    // VK_RBUTTON = 0x02. Check if high bit is set (key is down)
    let rmb_down = (GetAsyncKeyState(0x02) as u16 & 0x8000) != 0;

    if rmb_down {
        if !ACTIVE_RMB_CONFINED.swap(true, Ordering::Relaxed) {
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            if GetWindowRect(fg_hwnd, &mut rect) != 0 {
                ClipCursor(&rect);
            }
        }
    } else {
        if ACTIVE_RMB_CONFINED.swap(false, Ordering::Relaxed) {
            ClipCursor(std::ptr::null());
        }
    }
}

unsafe fn install_mouse_iat_hooks() {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    if base == 0 {
        return;
    }

    let dos_header = &*(base as *const IMAGE_DOS_HEADER);
    if dos_header.e_magic != 0x5A4D { // 'MZ'
        return;
    }

    let nt_headers_ptr = (base + dos_header.e_lfanew as usize) as *const u32;
    if *nt_headers_ptr != 0x00004550 { // 'PE\0\0'
        return;
    }

    let data_dir_ptr = (base + dos_header.e_lfanew as usize + 136) as *const IMAGE_DATA_DIRECTORY;
    let import_dir = &*data_dir_ptr.add(1);
    if import_dir.virtual_address == 0 || import_dir.size == 0 {
        return;
    }

    let user32_mod = LoadLibraryA(b"user32.dll\0".as_ptr());
    if user32_mod.is_null() {
        return;
    }
    let orig_set_cursor = GetProcAddress(user32_mod, b"SetCursorPos\0".as_ptr());
    if orig_set_cursor.is_null() {
        return;
    }
    REAL_SET_CURSOR_POS = Some(std::mem::transmute(orig_set_cursor));

    let mut desc = (base + import_dir.virtual_address as usize) as *const IMAGE_IMPORT_DESCRIPTOR;
    while (*desc).name != 0 {
        let name_ptr = (base + (*desc).name as usize) as *const i8;
        let mut is_user32 = false;
        let mut i = 0;
        let target = b"user32.dll";
        loop {
            let c = *name_ptr.add(i) as u8;
            if c == 0 {
                if i == target.len() {
                    is_user32 = true;
                }
                break;
            }
            if i >= target.len() || c.to_ascii_lowercase() != target[i] {
                break;
            }
            i += 1;
        }

        if is_user32 {
            let mut thunk = (base + (*desc).first_thunk as usize) as *mut usize;
            while *thunk != 0 {
                if *thunk == orig_set_cursor as usize {
                    let mut old_prot = 0;
                    if VirtualProtect(thunk as *mut c_void, std::mem::size_of::<usize>(), PAGE_EXECUTE_READWRITE, &mut old_prot) != 0 {
                        *thunk = Hooked_SetCursorPos as *const () as usize;
                        VirtualProtect(thunk as *mut c_void, std::mem::size_of::<usize>(), old_prot, &mut old_prot);
                    }
                    break;
                }
                thunk = thunk.add(1);
            }
        }
        desc = desc.add(1);
    }
}

// ---------------------------------------------------------------------------
// VMT HOOKS: SwapChain Present
// ---------------------------------------------------------------------------

unsafe extern "system" fn Hooked_Present(this: *mut c_void, SyncInterval: u32, Flags: u32) -> HRESULT {
    // Wayland camera rotation mouse confinement & warp handling
    handle_mouse_confinement();
    // 1. Inactive window GPU throttling
    if ALT_TAB_THROTTLE.load(Ordering::Relaxed) {
        let fg_hwnd = GetForegroundWindow();
        let mut fg_pid = 0;
        if !fg_hwnd.is_null() {
            GetWindowThreadProcessId(fg_hwnd, &mut fg_pid);
        }
        let my_pid = GetCurrentProcessId();

        // If another window (VS Code, browser, terminal) is active, insert sleep
        if fg_pid != 0 && fg_pid != my_pid {
            let sleep_ms = ALT_TAB_SLEEP_MS.load(Ordering::Relaxed);
            if sleep_ms > 0 {
                Sleep(sleep_ms);
            }
        }
    }

    // 2. Telemetry tracking
    record_frame_telemetry();

    // 3. Call original Present
    if let Some(orig) = REAL_PRESENT {
        orig(this, SyncInterval, Flags)
    } else {
        S_OK
    }
}

unsafe extern "system" fn Hooked_Present1(
    this: *mut c_void,
    SyncInterval: u32,
    Flags: u32,
    pPresentParameters: *const c_void,
) -> HRESULT {
    handle_mouse_confinement();
    if ALT_TAB_THROTTLE.load(Ordering::Relaxed) {
        let fg_hwnd = GetForegroundWindow();
        let mut fg_pid = 0;
        if !fg_hwnd.is_null() {
            GetWindowThreadProcessId(fg_hwnd, &mut fg_pid);
        }
        let my_pid = GetCurrentProcessId();

        if fg_pid != 0 && fg_pid != my_pid {
            let sleep_ms = ALT_TAB_SLEEP_MS.load(Ordering::Relaxed);
            if sleep_ms > 0 {
                Sleep(sleep_ms);
            }
        }
    }

    record_frame_telemetry();

    if let Some(orig) = REAL_PRESENT1 {
        orig(this, SyncInterval, Flags, pPresentParameters)
    } else {
        S_OK
    }
}

fn record_frame_telemetry() {
    let now = unsafe { GetTickCount64() };
    let last = LAST_TELEMETRY_TIME.load(Ordering::Relaxed);
    let frames = FRAME_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

    if last == 0 {
        LAST_TELEMETRY_TIME.store(now, Ordering::Relaxed);
        return;
    }

    let elapsed = now.saturating_sub(last);
    if elapsed >= 1000 {
        let fps = ((frames * 1000) / elapsed) as u32;
        CURRENT_FPS.store(fps, Ordering::Relaxed);
        FRAME_COUNT.store(0, Ordering::Relaxed);
        LAST_TELEMETRY_TIME.store(now, Ordering::Relaxed);

        // Write telemetry to shared file
        write_telemetry_file(fps);
    }
}

fn write_telemetry_file(fps: u32) {
    let cap_mb = VRAM_CAP_MB.load(Ordering::Relaxed);
    let json = format!(
        "{{\"fps\":{},\"vram_cap_mb\":{},\"alt_tab_throttled\":{}}}\n",
        fps,
        cap_mb,
        ALT_TAB_THROTTLE.load(Ordering::Relaxed)
    );

    // Try writing to both Linux tmp and local drive C
    let _ = std::fs::write("C:\\rtrust_gpu_stats.json", &json);
    let _ = std::fs::write("Z:\\tmp\\rtrust_gpu_stats.json", &json);
}

// ---------------------------------------------------------------------------
// VMT HOOKS: SwapChain Creation
// ---------------------------------------------------------------------------

unsafe fn hook_swap_chain(swapchain: *mut c_void) {
    if swapchain.is_null() {
        return;
    }
    let vtable = *(swapchain as *mut *mut usize);

    // Present is at index 8
    let orig = hook_vtable_entry(vtable, 8, Hooked_Present as *const () as usize);
    if REAL_PRESENT.is_none() {
        REAL_PRESENT = Some(std::mem::transmute(orig));
    }

    // Present1 is at index 22 (for IDXGISwapChain1)
    let orig1 = hook_vtable_entry(vtable, 22, Hooked_Present1 as *const () as usize);
    if REAL_PRESENT1.is_none() {
        REAL_PRESENT1 = Some(std::mem::transmute(orig1));
    }
}

unsafe extern "system" fn Hooked_CreateSwapChain(
    this: *mut c_void,
    pDevice: *mut c_void,
    pDesc: *mut c_void,
    ppSwapChain: *mut *mut c_void,
) -> HRESULT {
    let res = if let Some(orig) = REAL_CREATE_SWAP_CHAIN {
        orig(this, pDevice, pDesc, ppSwapChain)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppSwapChain.is_null() && !(*ppSwapChain).is_null() {
        hook_swap_chain(*ppSwapChain);
    }

    res
}

unsafe extern "system" fn Hooked_CreateSwapChainForHwnd(
    this: *mut c_void,
    pDevice: *mut c_void,
    hWnd: *mut c_void,
    pDesc: *const c_void,
    pFullscreenDesc: *const c_void,
    pRestrictToOutput: *const c_void,
    ppSwapChain: *mut *mut c_void,
) -> HRESULT {
    let res = if let Some(orig) = REAL_CREATE_SWAP_CHAIN_HWND {
        orig(this, pDevice, hWnd, pDesc, pFullscreenDesc, pRestrictToOutput, ppSwapChain)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppSwapChain.is_null() && !(*ppSwapChain).is_null() {
        hook_swap_chain(*ppSwapChain);
    }

    res
}

// ---------------------------------------------------------------------------
// VMT HOOKS: Adapter GetDesc (VRAM Capping)
// ---------------------------------------------------------------------------

unsafe extern "system" fn Hooked_GetDesc(this: *mut c_void, pDesc: *mut DXGI_ADAPTER_DESC) -> HRESULT {
    let res = if let Some(orig) = REAL_GET_DESC {
        orig(this, pDesc)
    } else {
        E_FAIL
    };

    if res == S_OK && !pDesc.is_null() {
        let cap_mb = VRAM_CAP_MB.load(Ordering::Relaxed);
        if cap_mb > 0 {
            let cap_bytes = (cap_mb as usize) * 1024 * 1024;
            if (*pDesc).DedicatedVideoMemory > cap_bytes {
                (*pDesc).DedicatedVideoMemory = cap_bytes;
            }
        }
    }

    res
}

unsafe extern "system" fn Hooked_GetDesc1(this: *mut c_void, pDesc: *mut DXGI_ADAPTER_DESC1) -> HRESULT {
    let res = if let Some(orig) = REAL_GET_DESC1 {
        orig(this, pDesc)
    } else {
        E_FAIL
    };

    if res == S_OK && !pDesc.is_null() {
        let cap_mb = VRAM_CAP_MB.load(Ordering::Relaxed);
        if cap_mb > 0 {
            let cap_bytes = (cap_mb as usize) * 1024 * 1024;
            if (*pDesc).DedicatedVideoMemory > cap_bytes {
                (*pDesc).DedicatedVideoMemory = cap_bytes;
            }
        }
    }

    res
}

unsafe fn hook_adapter(adapter: *mut c_void) {
    if adapter.is_null() {
        return;
    }
    let vtable = *(adapter as *mut *mut usize);

    // GetDesc is at index 8
    let orig = hook_vtable_entry(vtable, 8, Hooked_GetDesc as *const () as usize);
    if REAL_GET_DESC.is_none() {
        REAL_GET_DESC = Some(std::mem::transmute(orig));
    }

    // GetDesc1 is at index 10 (for IDXGIAdapter1)
    let orig1 = hook_vtable_entry(vtable, 10, Hooked_GetDesc1 as *const () as usize);
    if REAL_GET_DESC1.is_none() {
        REAL_GET_DESC1 = Some(std::mem::transmute(orig1));
    }
}

unsafe extern "system" fn Hooked_EnumAdapters(
    this: *mut c_void,
    Adapter: u32,
    ppAdapter: *mut *mut c_void,
) -> HRESULT {
    let res = if let Some(orig) = REAL_ENUM_ADAPTERS {
        orig(this, Adapter, ppAdapter)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppAdapter.is_null() && !(*ppAdapter).is_null() {
        hook_adapter(*ppAdapter);
    }

    res
}

unsafe extern "system" fn Hooked_EnumAdapters1(
    this: *mut c_void,
    Adapter: u32,
    ppAdapter: *mut *mut c_void,
) -> HRESULT {
    let res = if let Some(orig) = REAL_ENUM_ADAPTERS1 {
        orig(this, Adapter, ppAdapter)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppAdapter.is_null() && !(*ppAdapter).is_null() {
        hook_adapter(*ppAdapter);
    }

    res
}

unsafe fn hook_factory(factory: *mut c_void) {
    if factory.is_null() {
        return;
    }
    let vtable = *(factory as *mut *mut usize);

    // EnumAdapters (index 7)
    let orig_enum = hook_vtable_entry(vtable, 7, Hooked_EnumAdapters as *const () as usize);
    if REAL_ENUM_ADAPTERS.is_none() {
        REAL_ENUM_ADAPTERS = Some(std::mem::transmute(orig_enum));
    }

    // CreateSwapChain (index 10)
    let orig_sc = hook_vtable_entry(vtable, 10, Hooked_CreateSwapChain as *const () as usize);
    if REAL_CREATE_SWAP_CHAIN.is_none() {
        REAL_CREATE_SWAP_CHAIN = Some(std::mem::transmute(orig_sc));
    }

    // EnumAdapters1 (index 12 for IDXGIFactory1)
    let orig_enum1 = hook_vtable_entry(vtable, 12, Hooked_EnumAdapters1 as *const () as usize);
    if REAL_ENUM_ADAPTERS1.is_none() {
        REAL_ENUM_ADAPTERS1 = Some(std::mem::transmute(orig_enum1));
    }

    // CreateSwapChainForHwnd (index 15 for IDXGIFactory2)
    let orig_sch = hook_vtable_entry(vtable, 15, Hooked_CreateSwapChainForHwnd as *const () as usize);
    if REAL_CREATE_SWAP_CHAIN_HWND.is_none() {
        REAL_CREATE_SWAP_CHAIN_HWND = Some(std::mem::transmute(orig_sch));
    }
}

// ---------------------------------------------------------------------------
// PUBLIC EXPORTS OF DXGI.DLL
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "system" fn CreateDXGIFactory(
    riid: *const GUID,
    ppFactory: *mut *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    let res = if let Some(func) = REAL_CREATE_FACTORY {
        func(riid, ppFactory)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppFactory.is_null() && !(*ppFactory).is_null() {
        hook_factory(*ppFactory);
    }

    res
}

#[no_mangle]
pub unsafe extern "system" fn CreateDXGIFactory1(
    riid: *const GUID,
    ppFactory: *mut *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    let res = if let Some(func) = REAL_CREATE_FACTORY1 {
        func(riid, ppFactory)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppFactory.is_null() && !(*ppFactory).is_null() {
        hook_factory(*ppFactory);
    }

    res
}

#[no_mangle]
pub unsafe extern "system" fn CreateDXGIFactory2(
    flags: u32,
    riid: *const GUID,
    ppFactory: *mut *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    let res = if let Some(func) = REAL_CREATE_FACTORY2 {
        func(flags, riid, ppFactory)
    } else {
        E_FAIL
    };

    if res == S_OK && !ppFactory.is_null() && !(*ppFactory).is_null() {
        hook_factory(*ppFactory);
    }

    res
}

#[no_mangle]
pub unsafe extern "system" fn DXGIDeclareAdapterRemovalSupport() -> HRESULT {
    init_real_dxgi();

    if let Some(func) = REAL_DECLARE_ADAPTER_REMOVAL {
        func()
    } else {
        S_OK
    }
}

#[no_mangle]
pub unsafe extern "system" fn DXGID3D10CreateDevice(
    d3d10core: *mut c_void,
    factory: *mut c_void,
    adapter: *mut c_void,
    flags: u32,
    feature_levels: *const c_void,
    level_count: u32,
    device: *mut *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    if let Some(func) = REAL_D3D10_CREATE_DEVICE {
        func(d3d10core, factory, adapter, flags, feature_levels, level_count, device)
    } else {
        E_FAIL
    }
}

#[no_mangle]
pub unsafe extern "system" fn DXGID3D10RegisterLayers(
    layers: *const c_void,
    layer_count: u32,
) -> HRESULT {
    init_real_dxgi();

    if let Some(func) = REAL_D3D10_REGISTER_LAYERS {
        func(layers, layer_count)
    } else {
        S_OK
    }
}

#[no_mangle]
pub unsafe extern "system" fn DXGIGetDebugInterface1(
    flags: u32,
    riid: *const GUID,
    ppDebug: *mut *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    if let Some(func) = REAL_GET_DEBUG_INTERFACE1 {
        func(flags, riid, ppDebug)
    } else {
        E_FAIL
    }
}

#[no_mangle]
pub unsafe extern "system" fn DXGID3D10CreateLayeredDevice(
    a1: *mut c_void,
    a2: *mut c_void,
    a3: *mut c_void,
    a4: *mut c_void,
    a5: *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    if !REAL_DXGI_MODULE.is_null() {
        let p = GetProcAddress(REAL_DXGI_MODULE, b"DXGID3D10CreateLayeredDevice\0".as_ptr());
        if !p.is_null() {
            let func: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut c_void) -> HRESULT =
                std::mem::transmute(p);
            return func(a1, a2, a3, a4, a5);
        }
    }
    E_FAIL
}

#[no_mangle]
pub unsafe extern "system" fn DXGID3D10GetProviderCode(
    a1: *mut c_void,
) -> HRESULT {
    init_real_dxgi();

    if !REAL_DXGI_MODULE.is_null() {
        let p = GetProcAddress(REAL_DXGI_MODULE, b"DXGID3D10GetProviderCode\0".as_ptr());
        if !p.is_null() {
            let func: unsafe extern "system" fn(*mut c_void) -> HRESULT = std::mem::transmute(p);
            return func(a1);
        }
    }
    E_FAIL
}
