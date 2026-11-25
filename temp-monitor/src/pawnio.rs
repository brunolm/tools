//! Minimal binding to PawnIOLib (<https://pawnio.eu>), loaded at runtime from the PawnIO install.
//! API: `PawnIOLib.h` shipped with PawnIO 2.x.

use std::ffi::CStr;
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows_sys::Win32::Foundation::{E_ACCESSDENIED, FreeLibrary, HANDLE, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows_sys::core::HRESULT;

type OpenFn = unsafe extern "system" fn(*mut HANDLE) -> HRESULT;
type LoadFn = unsafe extern "system" fn(HANDLE, *const u8, usize) -> HRESULT;
type ExecuteFn = unsafe extern "system" fn(
    HANDLE,
    *const u8,
    *const u64,
    usize,
    *mut u64,
    usize,
    *mut usize,
) -> HRESULT;
type CloseFn = unsafe extern "system" fn(HANDLE) -> HRESULT;

#[derive(Debug)]
pub enum Error {
    NotInstalled,
    AccessDenied,
    Call {
        function: &'static str,
        hresult: HRESULT,
    },
}

/// An open PawnIO executor with one module loaded. Closed on drop.
pub struct PawnIo {
    lib: HMODULE,
    handle: HANDLE,
    execute: ExecuteFn,
    close: CloseFn,
}

impl PawnIo {
    /// Opens an executor and loads `module`, a signed PawnIO module blob.
    pub fn open(module: &[u8]) -> Result<Self, Error> {
        let lib = load_library().ok_or(Error::NotInstalled)?;
        let symbols = unsafe {
            (
                symbol::<OpenFn>(lib, c"pawnio_open"),
                symbol::<LoadFn>(lib, c"pawnio_load"),
                symbol::<ExecuteFn>(lib, c"pawnio_execute"),
                symbol::<CloseFn>(lib, c"pawnio_close"),
            )
        };
        let (Some(open), Some(load), Some(execute), Some(close)) = symbols else {
            unsafe { FreeLibrary(lib) };
            return Err(Error::NotInstalled);
        };

        let mut handle: HANDLE = std::ptr::null_mut();
        let hr = unsafe { open(&mut handle) };
        if hr < 0 {
            unsafe { FreeLibrary(lib) };
            return Err(match hr {
                E_ACCESSDENIED => Error::AccessDenied,
                _ => Error::Call {
                    function: "pawnio_open",
                    hresult: hr,
                },
            });
        }
        let pawn = Self {
            lib,
            handle,
            execute,
            close,
        };
        let hr = unsafe { load(handle, module.as_ptr(), module.len()) };
        if hr < 0 {
            return Err(Error::Call {
                function: "pawnio_load",
                hresult: hr,
            });
        }
        Ok(pawn)
    }

    /// Runs a module function on the calling thread's current CPU; returns the entries written.
    pub fn execute(&self, function: &CStr, input: &[u64], output: &mut [u64]) -> Option<usize> {
        let mut written = 0;
        let hr = unsafe {
            (self.execute)(
                self.handle,
                function.as_ptr().cast(),
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written,
            )
        };
        (hr >= 0).then_some(written)
    }
}

impl Drop for PawnIo {
    fn drop(&mut self) {
        unsafe {
            (self.close)(self.handle);
            FreeLibrary(self.lib);
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => write!(f, "PawnIO isn't installed."),
            Self::AccessDenied => write!(f, "PawnIO needs the app to run as administrator."),
            Self::Call { function, hresult } => {
                write!(f, "PawnIO {function} failed (HRESULT 0x{hresult:08X}).")
            }
        }
    }
}

fn load_library() -> Option<HMODULE> {
    let dir = std::env::var_os("ProgramFiles")?;
    let path = PathBuf::from(dir).join("PawnIO").join("PawnIOLib.dll");
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let lib = unsafe {
        LoadLibraryExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    };
    (!lib.is_null()).then_some(lib)
}

/// # Safety
/// `T` must be the `extern "system"` function pointer type matching the export's signature.
unsafe fn symbol<T>(lib: HMODULE, name: &CStr) -> Option<T> {
    let proc = unsafe { GetProcAddress(lib, name.as_ptr().cast()) }?;
    Some(unsafe { std::mem::transmute_copy(&proc) })
}
