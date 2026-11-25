//! GPU temperatures from the WDDM kernel thunk (`D3DKMT_ADAPTER_PERFDATA`), the same
//! vendor-neutral source Task Manager uses. Requires a WDDM 2.4+ driver that reports it.

use std::ffi::c_void;

use windows_sys::Wdk::Graphics::Direct3D::{
    D3DKMT_ADAPTER_PERFDATA, D3DKMT_ADAPTERINFO, D3DKMT_CLOSEADAPTER, D3DKMT_DRIVER_DESCRIPTION,
    D3DKMT_ENUMADAPTERS2, D3DKMT_QUERYADAPTERINFO, D3DKMTCloseAdapter, D3DKMTEnumAdapters2,
    D3DKMTQueryAdapterInfo, KMTQAITYPE_ADAPTERPERFDATA, KMTQAITYPE_DRIVER_DESCRIPTION,
    KMTQUERYADAPTERINFOTYPE,
};

const BASIC_RENDER_DRIVER: &str = "Microsoft Basic Render Driver";

pub struct GpuReading {
    pub name: String,
    /// `None` when the driver doesn't report a temperature (typical for integrated GPUs).
    pub temp_c: Option<f32>,
}

pub fn read() -> Vec<GpuReading> {
    let adapters = enum_adapters();
    let readings = adapters
        .iter()
        .filter_map(|adapter| {
            let name = driver_description(adapter.hAdapter)?;
            if name == BASIC_RENDER_DRIVER {
                return None;
            }
            let temp_c = perf_data(adapter.hAdapter)
                .filter(|p| p.Temperature > 0)
                .map(|p| p.Temperature as f32 / 10.0);
            Some(GpuReading { name, temp_c })
        })
        .collect();
    for adapter in &adapters {
        let close = D3DKMT_CLOSEADAPTER {
            hAdapter: adapter.hAdapter,
        };
        unsafe { D3DKMTCloseAdapter(&close) };
    }
    readings
}

fn enum_adapters() -> Vec<D3DKMT_ADAPTERINFO> {
    let mut query = D3DKMT_ENUMADAPTERS2 {
        NumAdapters: 0,
        pAdapters: std::ptr::null_mut(),
    };
    if unsafe { D3DKMTEnumAdapters2(&mut query) } < 0 || query.NumAdapters == 0 {
        return Vec::new();
    }
    let mut adapters: Vec<D3DKMT_ADAPTERINFO> =
        vec![unsafe { std::mem::zeroed() }; query.NumAdapters as usize];
    query.pAdapters = adapters.as_mut_ptr();
    if unsafe { D3DKMTEnumAdapters2(&mut query) } < 0 {
        return Vec::new();
    }
    adapters.truncate(query.NumAdapters as usize);
    adapters
}

fn driver_description(adapter: u32) -> Option<String> {
    let mut desc: D3DKMT_DRIVER_DESCRIPTION = unsafe { std::mem::zeroed() };
    query_adapter(adapter, KMTQAITYPE_DRIVER_DESCRIPTION, &mut desc)?;
    let raw = &desc.DriverDescription;
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    Some(String::from_utf16_lossy(&raw[..end]))
}

fn perf_data(adapter: u32) -> Option<D3DKMT_ADAPTER_PERFDATA> {
    let mut perf: D3DKMT_ADAPTER_PERFDATA = unsafe { std::mem::zeroed() };
    query_adapter(adapter, KMTQAITYPE_ADAPTERPERFDATA, &mut perf)?;
    Some(perf)
}

fn query_adapter<T>(adapter: u32, kind: KMTQUERYADAPTERINFOTYPE, out: &mut T) -> Option<()> {
    let mut query = D3DKMT_QUERYADAPTERINFO {
        hAdapter: adapter,
        Type: kind,
        pPrivateDriverData: (out as *mut T).cast::<c_void>(),
        PrivateDriverDataSize: size_of::<T>() as u32,
    };
    (unsafe { D3DKMTQueryAdapterInfo(&mut query) } >= 0).then_some(())
}
