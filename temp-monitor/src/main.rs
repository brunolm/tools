#![windows_subsystem = "windows"]

mod app;
mod cpu;
mod gpu;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Temp Monitor")
            .with_inner_size([520.0, 640.0])
            .with_min_inner_size([360.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Temp Monitor",
        options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}
