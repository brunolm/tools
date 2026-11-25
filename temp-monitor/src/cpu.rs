//! CPU temperatures from Core Temp's shared-memory plugin interface.
//! Layout: <https://www.alcpu.com/CoreTemp/developers.html> (`CORE_TEMP_SHARED_DATA`).

use std::path::PathBuf;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Memory::{
    FILE_MAP_READ, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWMINNOACTIVE;

const MAPPING_NAME: &str = "CoreTempMappingObject";

#[repr(C)]
#[derive(Clone, Copy)]
struct SharedData {
    load: [u32; 256],
    tj_max: [u32; 128],
    core_count: u32,
    cpu_count: u32,
    temp: [f32; 256],
    vid: f32,
    cpu_speed: f32,
    fsb_speed: f32,
    multiplier: f32,
    cpu_name: [u8; 100],
    fahrenheit: u8,
    delta_to_tj_max: u8,
}

pub struct CpuReading {
    pub name: String,
    pub tj_max_c: f32,
    pub cores: Vec<CoreReading>,
}

pub struct CoreReading {
    pub temp_c: f32,
    pub load_pct: u32,
}

impl CpuReading {
    pub fn hottest_c(&self) -> Option<f32> {
        self.cores.iter().map(|c| c.temp_c).reduce(f32::max)
    }
}

/// Returns one reading per physical CPU package, or `None` when Core Temp isn't running.
pub fn read() -> Option<Vec<CpuReading>> {
    let data = read_shared()?;
    let core_count = data.core_count as usize;
    let cpu_count = data.cpu_count as usize;
    if core_count == 0 || cpu_count == 0 || core_count * cpu_count > data.temp.len() {
        return None;
    }

    let name = cpu_name(&data.cpu_name);
    let to_celsius = |v: f32| {
        if data.fahrenheit != 0 {
            (v - 32.0) * 5.0 / 9.0
        } else {
            v
        }
    };
    let readings = (0..cpu_count.min(data.tj_max.len()))
        .map(|cpu| {
            let tj_max = data.tj_max[cpu] as f32;
            let cores = (0..core_count)
                .map(|core| {
                    let i = cpu * core_count + core;
                    let raw = data.temp[i];
                    // In "distance to TjMax" mode Core Temp stores TjMax minus the temperature.
                    let temp = if data.delta_to_tj_max != 0 {
                        tj_max - raw
                    } else {
                        raw
                    };
                    CoreReading {
                        temp_c: to_celsius(temp),
                        load_pct: data.load[i],
                    }
                })
                .collect();
            CpuReading {
                name: name.clone(),
                tj_max_c: to_celsius(tj_max),
                cores,
            }
        })
        .collect();
    Some(readings)
}

pub fn core_temp_exe() -> Option<PathBuf> {
    let base = std::env::var_os("ProgramFiles")?;
    let exe = PathBuf::from(base).join("Core Temp").join("Core Temp.exe");
    exe.is_file().then_some(exe)
}

/// Launches Core Temp through the shell so Windows shows its UAC prompt; it requires elevation.
pub fn start_core_temp(exe: &std::path::Path) {
    let file = wide(&exe.to_string_lossy());
    let verb = wide("open");
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWMINNOACTIVE,
        );
    }
}

fn read_shared() -> Option<SharedData> {
    let name = wide(MAPPING_NAME);
    unsafe {
        let mapping = OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr());
        if mapping.is_null() {
            return None;
        }
        let view = MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, size_of::<SharedData>());
        let data = (!view.Value.is_null())
            .then(|| std::ptr::read_unaligned(view.Value as *const SharedData));
        if !view.Value.is_null() {
            UnmapViewOfFile(view);
        }
        CloseHandle(mapping);
        data
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn cpu_name(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_data_matches_core_temp_layout() {
        assert_eq!(size_of::<SharedData>(), 2688);
    }

    #[test]
    fn cpu_name_stops_at_nul_and_trims() {
        let mut raw = [0u8; 100];
        raw[..8].copy_from_slice(b" i9 CPU ");
        assert_eq!(cpu_name(&raw), "i9 CPU");
    }
}
