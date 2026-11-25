#![windows_subsystem = "windows"]

mod app;
mod cpu;
mod gpu;
mod pawnio;

use eframe::egui;
use eframe::egui_wgpu::WgpuSetup;

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Temp Monitor")
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([360.0, 240.0]),
        ..Default::default()
    };
    // eframe defaults to the high-performance adapter, which keeps a discrete GPU awake.
    if let WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup {
        setup.power_preference = eframe::wgpu::PowerPreference::LowPower;
    }
    eframe::run_native(
        "Temp Monitor",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
