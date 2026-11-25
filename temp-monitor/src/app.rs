use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, ProgressBar, RichText, Ui};

use crate::cpu::{self, CoreKind};
use crate::{gpu, pawnio};

const REFRESH: Duration = Duration::from_secs(1);
const GPU_BAR_MAX_C: f32 = 100.0;
const WARM_C: f32 = 60.0;
const HOT_C: f32 = 80.0;

pub struct App {
    cpu: Result<cpu::Monitor, cpu::Error>,
    cpu_reading: Option<cpu::CpuReading>,
    gpus: Vec<gpu::GpuReading>,
    last_refresh: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            cpu: cpu::Monitor::open(),
            cpu_reading: None,
            gpus: Vec::new(),
            last_refresh: None,
        }
    }

    fn refresh_if_due(&mut self) {
        if self.last_refresh.is_some_and(|t| t.elapsed() < REFRESH) {
            return;
        }
        if self.cpu.is_err() {
            self.cpu = cpu::Monitor::open();
        }
        self.cpu_reading = self.cpu.as_ref().ok().map(cpu::Monitor::read);
        self.gpus = gpu::read();
        self.last_refresh = Some(Instant::now());
    }

    fn cpu_section(&self, ui: &mut Ui) {
        ui.heading("CPU");
        if let Err(e) = &self.cpu {
            cpu_unavailable(ui, e);
            return;
        }
        if let Some(reading) = &self.cpu_reading {
            cpu_package(ui, reading);
        }
    }

    fn gpu_section(&self, ui: &mut Ui) {
        ui.heading("GPU");
        if self.gpus.is_empty() {
            ui.label("No GPUs found.");
            return;
        }
        egui::Grid::new("gpus")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for gpu in &self.gpus {
                    ui.label(&gpu.name);
                    match gpu.temp_c {
                        Some(t) => temp_bar(ui, t, GPU_BAR_MAX_C),
                        None => {
                            ui.weak("not reported by the driver");
                        }
                    }
                    ui.end_row();
                }
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.refresh_if_due();
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.cpu_section(ui);
                ui.add_space(12.0);
                ui.separator();
                self.gpu_section(ui);
            });
        });
        ui.ctx().request_repaint_after(REFRESH);
    }
}

fn cpu_unavailable(ui: &mut Ui, error: &cpu::Error) {
    ui.label(error.to_string());
    if matches!(error, cpu::Error::PawnIo(pawnio::Error::NotInstalled)) {
        ui.label("Install it, and the CPU temperatures will show up here:");
        ui.code("winget install namazso.PawnIO");
    }
}

fn cpu_package(ui: &mut Ui, reading: &cpu::CpuReading) {
    ui.label(RichText::new(&reading.name).strong());
    ui.horizontal(|ui| {
        if let Some(hottest) = reading.hottest_c() {
            ui.label(
                RichText::new(format!("{hottest:.0} °C"))
                    .size(22.0)
                    .color(temp_color(hottest)),
            );
            ui.weak("hottest core");
        }
        if let Some(package) = reading.package_c {
            ui.label(format!("Package {package:.0} °C"));
        }
        ui.weak(format!("TjMax {:.0} °C", reading.tj_max_c));
    });
    egui::Grid::new("cores")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (i, core) in reading.cores.iter().enumerate() {
                ui.label(format!("Core #{i}"));
                ui.weak(kind_label(core.kind));
                match core.temp_c {
                    Some(t) => temp_bar(ui, t, reading.tj_max_c),
                    None => {
                        ui.weak("no reading");
                    }
                }
                ui.end_row();
            }
        });
}

fn temp_bar(ui: &mut Ui, temp_c: f32, max_c: f32) {
    let fraction = if max_c > 0.0 { temp_c / max_c } else { 0.0 };
    ui.add(
        ProgressBar::new(fraction.clamp(0.0, 1.0))
            .desired_width(220.0)
            .fill(temp_color(temp_c))
            .text(RichText::new(format!("{temp_c:.0} °C")).color(Color32::BLACK)),
    );
}

fn temp_color(temp_c: f32) -> Color32 {
    if temp_c < WARM_C {
        Color32::from_rgb(0x4c, 0xaf, 0x50)
    } else if temp_c < HOT_C {
        Color32::from_rgb(0xff, 0xb3, 0x00)
    } else {
        Color32::from_rgb(0xe5, 0x39, 0x35)
    }
}

fn kind_label(kind: CoreKind) -> &'static str {
    match kind {
        CoreKind::Performance => "P",
        CoreKind::Efficient => "E",
        CoreKind::Uniform => "",
    }
}
