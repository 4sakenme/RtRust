//! Mock loader implementation for Windows applications running under Wine.
//!
//! Intercepts loader initialization calls and stubs out environment creation
//! to prevent launching background web helper processes on memory-constrained systems.

#![cfg_attr(all(not(test), target_os = "windows"), no_std)]
#![allow(non_snake_case)]

use core::ffi::c_void;

#[cfg(all(not(test), target_os = "windows"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;
pub const E_NOTIMPL: HRESULT = -2147467263; // 0x80004001
pub const HRESULT_CLASS_NOT_AVAILABLE: HRESULT = -2147221164; // 0x80040154 (REGDB_E_CLASSNOTREG)

/// Attempts to locate installed browser runtime version string.
/// Returning an error code signals to the host that no runtime is available.
#[no_mangle]
pub unsafe extern "system" fn GetAvailableCoreWebView2BrowserVersionString(
    _browser_executable_folder: *const u16,
    version_info: *mut *mut u16,
) -> HRESULT {
    if !version_info.is_null() {
        *version_info = core::ptr::null_mut();
    }
    // Return class not registered so host knows no chromium runtime is bundled
    HRESULT_CLASS_NOT_AVAILABLE
}

/// Variant with client options parameter.
#[no_mangle]
pub unsafe extern "system" fn GetAvailableCoreWebView2BrowserVersionStringWithOptions(
    _browser_executable_folder: *const u16,
    _environment_options: *mut c_void,
    version_info: *mut *mut u16,
) -> HRESULT {
    if !version_info.is_null() {
        *version_info = core::ptr::null_mut();
    }
    HRESULT_CLASS_NOT_AVAILABLE
}

/// Stub environment initialization call.
#[no_mangle]
pub unsafe extern "system" fn CreateCoreWebView2EnvironmentWithOptions(
    _browser_executable_folder: *const u16,
    _user_data_folder: *const u16,
    _environment_options: *mut c_void,
    _environment_created_handler: *mut c_void,
) -> HRESULT {
    // Return not implemented or class not available
    E_NOTIMPL
}

/// Legacy environment creation signature without extra options.
#[no_mangle]
pub unsafe extern "system" fn CreateCoreWebView2Environment(
    _environment_created_handler: *mut c_void,
) -> HRESULT {
    E_NOTIMPL
}

/// Version comparison utility.
#[no_mangle]
pub unsafe extern "system" fn CompareBrowserVersions(
    _version1: *const u16,
    _version2: *const u16,
    result: *mut i32,
) -> HRESULT {
    if !result.is_null() {
        *result = 0;
    }
    S_OK
}
