#![windows_subsystem = "windows"]

mod placement;
mod tools;

use std::cell::Cell;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreateSolidBrush, DeleteObject, HFONT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::SetWindowTheme;
use windows_sys::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForWindow, SystemParametersInfoForDpi,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::w;

use tools::TOOLS;

const MARGIN: i32 = 12;
const GAP: i32 = 8;
const BUTTON_W: i32 = 240;
const BUTTON_H: i32 = 34;
/// `0x00BBGGRR`, the dark-theme window background.
const BACKGROUND: COLORREF = 0x0020_2020;
const WINDOW_STYLE: WINDOW_STYLE = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

thread_local! {
    static FONT: Cell<HFONT> = const { Cell::new(null_mut()) };
}

fn main() {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = w!("ToolsLauncher");
        let window_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hIcon: LoadIconW(instance, 1 as _),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: CreateSolidBrush(BACKGROUND),
            lpszClassName: class,
            ..zeroed()
        };
        RegisterClassExW(&window_class);

        let hwnd = CreateWindowExW(
            0,
            class,
            w!("Tools"),
            WINDOW_STYLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        let dark: i32 = 1;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            &dark as *const i32 as *const c_void,
            size_of::<i32>() as u32,
        );
        create_buttons(hwnd);
        refresh_buttons(hwnd);
        placement::restore(hwnd);
        // The saved size may come from another DPI or font size, so size for where it landed.
        layout(hwnd, GetDpiForWindow(hwnd), None);
        ShowWindow(hwnd, SW_SHOW);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(hwnd, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND if hiword(wparam) == BN_CLICKED => {
                launch(hwnd, loword(wparam));
                0
            }
            // Re-check on focus so tools built while the launcher was open become clickable.
            WM_ACTIVATE if loword(wparam) != WA_INACTIVE => {
                refresh_buttons(hwnd);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            // Paints the corners around the rounded buttons; the default is the light button face.
            WM_CTLCOLORBTN => GetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND) as LRESULT,
            WM_DPICHANGED => {
                let suggested = &*(lparam as *const RECT);
                layout(hwnd, loword(wparam), Some((suggested.left, suggested.top)));
                0
            }
            WM_DESTROY => {
                placement::save(hwnd);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn create_buttons(hwnd: HWND) {
    for id in 1..=TOOLS.len() {
        unsafe {
            let button = CreateWindowExW(
                0,
                w!("BUTTON"),
                null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                0,
                0,
                0,
                0,
                hwnd,
                id as _,
                GetModuleHandleW(null()),
                null(),
            );
            SetWindowTheme(button, w!("DarkMode_Explorer"), null());
        }
    }
}

/// Sizes the font, buttons and window for `dpi`; moves the window to `origin` when given.
unsafe fn layout(hwnd: HWND, dpi: u32, origin: Option<(i32, i32)>) {
    let scale = |v: i32| v * dpi as i32 / 96;
    let font = unsafe { message_font(dpi) };
    for (i, id) in (1..=TOOLS.len()).enumerate() {
        let top = MARGIN + i as i32 * (BUTTON_H + GAP);
        unsafe {
            let button = GetDlgItem(hwnd, id as i32);
            SendMessageW(button, WM_SETFONT, font as WPARAM, 1);
            SetWindowPos(
                button,
                null_mut(),
                scale(MARGIN),
                scale(top),
                scale(BUTTON_W),
                scale(BUTTON_H),
                SWP_NOZORDER,
            );
        }
    }
    let old_font = FONT.replace(font);
    if !old_font.is_null() {
        unsafe { DeleteObject(old_font) };
    }

    let count = TOOLS.len() as i32;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scale(BUTTON_W + 2 * MARGIN),
        bottom: scale(2 * MARGIN + count * BUTTON_H + (count - 1) * GAP),
    };
    unsafe {
        AdjustWindowRectExForDpi(&mut rect, WINDOW_STYLE, 0, 0, dpi);
        let (x, y) = origin.unwrap_or_default();
        let flags = if origin.is_some() {
            SWP_NOZORDER
        } else {
            SWP_NOZORDER | SWP_NOMOVE
        };
        SetWindowPos(
            hwnd,
            null_mut(),
            x,
            y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            flags,
        );
    }
}

/// The system UI font at `dpi`; the caller owns the returned handle.
unsafe fn message_font(dpi: u32) -> HFONT {
    unsafe {
        let mut metrics: NONCLIENTMETRICSW = zeroed();
        metrics.cbSize = size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            &mut metrics as *mut _ as *mut c_void,
            0,
            dpi,
        );
        CreateFontIndirectW(&metrics.lfMessageFont)
    }
}

unsafe fn launch(hwnd: HWND, id: u32) {
    let Some(tool) = TOOLS.get(id as usize - 1) else {
        return;
    };
    let Err(err) = tool.launch() else {
        return;
    };
    let text = wide(&format!("Couldn't start {}:\n{err}", tool.name));
    unsafe { MessageBoxW(hwnd, text.as_ptr(), w!("Tools"), MB_OK | MB_ICONERROR) };
}

unsafe fn refresh_buttons(hwnd: HWND) {
    for (i, tool) in TOOLS.iter().enumerate() {
        let built = tool.is_built();
        let label = if built {
            tool.name.to_owned()
        } else {
            format!("{} (not built)", tool.name)
        };
        let label = wide(&label);
        unsafe {
            let button = GetDlgItem(hwnd, i as i32 + 1);
            SetWindowTextW(button, label.as_ptr());
            EnableWindow(button, built as i32);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn loword(v: usize) -> u32 {
    (v & 0xFFFF) as u32
}

fn hiword(v: usize) -> u32 {
    ((v >> 16) & 0xFFFF) as u32
}
