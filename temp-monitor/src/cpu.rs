//! CPU temperatures from Intel's per-core digital thermal sensors, read through PawnIO.
//! MSR layouts: Intel SDM Vol. 4 (IA32_THERM_STATUS, IA32_PACKAGE_THERM_STATUS,
//! MSR_TEMPERATURE_TARGET).

use std::arch::x86_64::__cpuid;
use std::ffi::CStr;
use std::fmt;
use std::mem::offset_of;

use windows_sys::Win32::System::SystemInformation::{
    GROUP_AFFINITY, GetLogicalProcessorInformationEx, PROCESSOR_RELATIONSHIP,
    RelationProcessorCore, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadGroupAffinity};

use crate::pawnio::{self, PawnIo};

/// Signed module from PawnIO.Modules 0.2.12 (LGPL-2.1, see `pawnio/COPYING`).
const INTEL_MSR_MODULE: &[u8] = include_bytes!("../pawnio/IntelMSR.bin");
const READ_MSR: &CStr = c"ioctl_read_msr";

const IA32_THERM_STATUS: u64 = 0x19C;
const IA32_PACKAGE_THERM_STATUS: u64 = 0x1B1;
const MSR_TEMPERATURE_TARGET: u64 = 0x1A2;
const READING_VALID: u64 = 1 << 31;
const FALLBACK_TJ_MAX_C: u8 = 100;

#[derive(Debug)]
pub enum Error {
    PawnIo(pawnio::Error),
    UnsupportedCpu(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoreKind {
    Performance,
    Efficient,
    Uniform,
}

pub struct CpuReading {
    pub name: String,
    pub tj_max_c: f32,
    pub package_c: Option<f32>,
    pub cores: Vec<CoreReading>,
}

pub struct CoreReading {
    pub kind: CoreKind,
    pub temp_c: Option<f32>,
}

/// Holds the PawnIO session; MSR reads are pinned to each physical core in turn.
pub struct Monitor {
    pawn: PawnIo,
    name: String,
    cores: Vec<Core>,
}

struct Core {
    affinity: GROUP_AFFINITY,
    kind: CoreKind,
    tj_max: u8,
}

impl CpuReading {
    pub fn hottest_c(&self) -> Option<f32> {
        self.cores.iter().filter_map(|c| c.temp_c).reduce(f32::max)
    }
}

impl Monitor {
    pub fn open() -> Result<Self, Error> {
        let vendor = cpu_vendor();
        if vendor != "GenuineIntel" {
            return Err(Error::UnsupportedCpu(vendor));
        }
        let pawn = PawnIo::open(INTEL_MSR_MODULE).map_err(Error::PawnIo)?;
        let topology = physical_cores();
        let affinities: Vec<_> = topology.iter().map(|(affinity, _)| *affinity).collect();
        let tj_max = on_each_core(&affinities, |_| {
            read_msr(&pawn, MSR_TEMPERATURE_TARGET).map(tj_max_from_target)
        });
        let cores = topology
            .into_iter()
            .zip(tj_max)
            .map(|((affinity, kind), tj_max)| Core {
                affinity,
                kind,
                tj_max: tj_max.unwrap_or(FALLBACK_TJ_MAX_C),
            })
            .collect();
        Ok(Self {
            pawn,
            name: cpu_brand(),
            cores,
        })
    }

    pub fn read(&self) -> CpuReading {
        let affinities: Vec<_> = self.cores.iter().map(|c| c.affinity).collect();
        let temps = on_each_core(&affinities, |i| {
            let status = read_msr(&self.pawn, IA32_THERM_STATUS)?;
            core_temp(status, self.cores[i].tj_max)
        });
        let package_tj_max = self.cores.first().map_or(FALLBACK_TJ_MAX_C, |c| c.tj_max);
        let package_c = read_msr(&self.pawn, IA32_PACKAGE_THERM_STATUS)
            .map(|status| below_tj_max(status, package_tj_max));
        CpuReading {
            name: self.name.clone(),
            tj_max_c: f32::from(package_tj_max),
            package_c,
            cores: self
                .cores
                .iter()
                .zip(temps)
                .map(|(core, temp_c)| CoreReading {
                    kind: core.kind,
                    temp_c,
                })
                .collect(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PawnIo(e) => e.fmt(f),
            Self::UnsupportedCpu(vendor) => {
                write!(f, "Only Intel CPUs are supported (this one is {vendor}).")
            }
        }
    }
}

/// Runs `f` with the current thread pinned to each affinity in turn, then restores the
/// original affinity. Yields `None` for cores the thread couldn't be pinned to.
fn on_each_core<T>(
    affinities: &[GROUP_AFFINITY],
    mut f: impl FnMut(usize) -> Option<T>,
) -> Vec<Option<T>> {
    let thread = unsafe { GetCurrentThread() };
    let mut original: GROUP_AFFINITY = unsafe { std::mem::zeroed() };
    let mut saved = false;
    let mut results = Vec::with_capacity(affinities.len());
    for (i, affinity) in affinities.iter().enumerate() {
        let previous = if saved {
            std::ptr::null_mut()
        } else {
            &mut original
        };
        let pinned = unsafe { SetThreadGroupAffinity(thread, affinity, previous) } != 0;
        saved |= pinned;
        results.push(if pinned { f(i) } else { None });
    }
    if saved {
        unsafe { SetThreadGroupAffinity(thread, &original, std::ptr::null_mut()) };
    }
    results
}

fn read_msr(pawn: &PawnIo, msr: u64) -> Option<u64> {
    let mut out = [0u64];
    let written = pawn.execute(READ_MSR, &[msr], &mut out)?;
    (written == 1).then_some(out[0])
}

fn cpu_vendor() -> String {
    let r = __cpuid(0);
    let bytes: Vec<u8> = [r.ebx, r.edx, r.ecx]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn physical_cores() -> Vec<(GROUP_AFFINITY, CoreKind)> {
    let mut len = 0u32;
    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, std::ptr::null_mut(), &mut len)
    };
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    let ok = unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, buf.as_mut_ptr().cast(), &mut len)
    };
    if ok == 0 {
        return Vec::new();
    }

    // Entries are variable-sized and shorter than the full union, so read only the fields used.
    let base = buf.as_ptr().cast::<u8>();
    let mut cores = Vec::new();
    let mut offset = 0usize;
    while offset < len as usize {
        let entry = unsafe { base.add(offset) };
        let size = unsafe {
            entry
                .add(offset_of!(SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, Size))
                .cast::<u32>()
                .read_unaligned()
        };
        let core = unsafe {
            entry
                .add(offset_of!(
                    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
                    Anonymous
                ))
                .cast::<PROCESSOR_RELATIONSHIP>()
                .read_unaligned()
        };
        cores.push((core.GroupMask[0], core.EfficiencyClass));
        offset += size as usize;
    }
    classify(cores)
}

/// Hybrid CPUs report a higher efficiency class for P-cores; uniform CPUs report one class.
fn classify(cores: Vec<(GROUP_AFFINITY, u8)>) -> Vec<(GROUP_AFFINITY, CoreKind)> {
    let max = cores.iter().map(|(_, class)| *class).max();
    let hybrid = cores.iter().any(|(_, class)| Some(*class) != max);
    cores
        .into_iter()
        .map(|(affinity, class)| {
            let kind = match (hybrid, Some(class) == max) {
                (false, _) => CoreKind::Uniform,
                (true, true) => CoreKind::Performance,
                (true, false) => CoreKind::Efficient,
            };
            (affinity, kind)
        })
        .collect()
}

fn cpu_brand() -> String {
    let bytes: Vec<u8> = (0x8000_0002..=0x8000_0004u32)
        .flat_map(|leaf| {
            let r = __cpuid(leaf);
            [r.eax, r.ebx, r.ecx, r.edx]
        })
        .flat_map(u32::to_le_bytes)
        .collect();
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn core_temp(status: u64, tj_max: u8) -> Option<f32> {
    (status & READING_VALID != 0).then(|| below_tj_max(status, tj_max))
}

/// Thermal status MSRs report the distance below TjMax in bits 22:16.
fn below_tj_max(status: u64, tj_max: u8) -> f32 {
    f32::from(tj_max) - ((status >> 16) & 0x7F) as f32
}

fn tj_max_from_target(target: u64) -> u8 {
    match ((target >> 16) & 0xFF) as u8 {
        0 => FALLBACK_TJ_MAX_C,
        tj_max => tj_max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn affinity(mask: usize) -> GROUP_AFFINITY {
        GROUP_AFFINITY {
            Mask: mask,
            Group: 0,
            Reserved: [0; 3],
        }
    }

    #[test]
    fn core_temp_subtracts_readout_from_tj_max() {
        let status = READING_VALID | (38 << 16);
        assert_eq!(core_temp(status, 100), Some(62.0));
    }

    #[test]
    fn core_temp_ignores_invalid_reading() {
        assert_eq!(core_temp(38 << 16, 100), None);
    }

    #[test]
    fn tj_max_reads_bits_23_16_and_falls_back_when_zero() {
        assert_eq!(tj_max_from_target(0x0064_0000), 100);
        assert_eq!(tj_max_from_target(0x0F5F_0000), 95);
        assert_eq!(tj_max_from_target(0), FALLBACK_TJ_MAX_C);
    }

    #[test]
    fn classify_splits_hybrid_cores_by_efficiency_class() {
        let kinds: Vec<_> = classify(vec![(affinity(0b11), 1), (affinity(0b100), 0)])
            .into_iter()
            .map(|(_, kind)| kind)
            .collect();
        assert_eq!(kinds, [CoreKind::Performance, CoreKind::Efficient]);
    }

    #[test]
    fn classify_marks_single_class_as_uniform() {
        let kinds: Vec<_> = classify(vec![(affinity(1), 0), (affinity(2), 0)])
            .into_iter()
            .map(|(_, kind)| kind)
            .collect();
        assert_eq!(kinds, [CoreKind::Uniform, CoreKind::Uniform]);
    }
}
