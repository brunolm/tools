use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_BINARY, RRF_RT_REG_BINARY, RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SW_HIDE, SetWindowPlacement, WINDOWPLACEMENT,
};
use windows_sys::core::PCWSTR;
use windows_sys::w;

const KEY: PCWSTR = w!("Software\\brunolm\\tools");
const VALUE: PCWSTR = w!("WindowPlacement");

/// Moves the still-hidden `hwnd` to where it was when it last closed; a no-op on first run.
/// `SetWindowPlacement` pulls the window back on screen if that spot is no longer visible.
pub unsafe fn restore(hwnd: HWND) {
    let mut placement: WINDOWPLACEMENT = unsafe { zeroed() };
    let mut size = size_of::<WINDOWPLACEMENT>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            KEY,
            VALUE,
            RRF_RT_REG_BINARY,
            null_mut(),
            &mut placement as *mut _ as *mut c_void,
            &mut size,
        )
    };
    if status != ERROR_SUCCESS || size != size_of::<WINDOWPLACEMENT>() as u32 {
        return;
    }
    placement.length = size;
    placement.showCmd = SW_HIDE as u32;
    unsafe { SetWindowPlacement(hwnd, &placement) };
}

pub unsafe fn save(hwnd: HWND) {
    let mut placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..unsafe { zeroed() }
    };
    if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
        return;
    }
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            KEY,
            VALUE,
            REG_BINARY,
            &placement as *const _ as *const c_void,
            placement.length,
        )
    };
}
