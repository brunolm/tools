use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, ProgressBar, RichText, Ui};

use crate::{cpu, gpu};

const REFRESH: Duration = Duration::from_secs(1);
const GPU_BAR_MAX_C: f32 = 100.0;
const WARM_C: f32 = 60.0;
const HOT_C: f32 = 80.0;

pub struct App {
    cpus: Option<Vec<cpu::CpuReading>>,
    gpus: Vec<gpu::GpuReading>,
    last_refresh: Option<Instant>,
    core_temp_exe: Option<PathBuf>,
}

impl App {
    pub fn new() -> Self {
        Self {
            cpus: None,
            gpus: Vec::new(),
            last_refresh: None,
            core_temp_exe: cpu::core_temp_exe(),
        }
    }

    fn refresh_if_due(&mut self) {
        if self.last_refresh.is_some_and(|t| t.elapsed() < REFRESH) {
            return;
        }
        self.cpus = cpu::read();
        self.gpus = gpu::read();
        self.last_refresh = Some(Instant::now());
    }

    fn cpu_section(&self, ui: &mut Ui) {
        ui.heading("CPU");
        let Some(cpus) = &self.cpus else {
            self.core_temp_missing(ui);
            return;
        };
        for (i, reading) in cpus.iter().enumerate() {
            cpu_package(ui, i, reading);
        }
    }

    fn core_temp_missing(&self, ui: &mut Ui) {
        ui.label("CPU temperatures come from Core Temp, which isn't running.");
        match &self.core_temp_exe {
            Some(exe) => {
                if ui.button("Start Core Temp").clicked() {
                    cpu::start_core_temp(exe);
                }
            }
            None => {
                ui.label("Install it with:");
                ui.code("winget install ALCPU.CoreTemp");
            }
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

fn cpu_package(ui: &mut Ui, index: usize, reading: &cpu::CpuReading) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(&reading.name).strong());
        if let Some(hottest) = reading.hottest_c() {
            ui.label(
                RichText::new(format!("{hottest:.0} °C"))
                    .size(22.0)
                    .color(temp_color(hottest)),
            );
        }
        ui.weak(format!("TjMax {:.0} °C", reading.tj_max_c));
    });
    egui::Grid::new(("cores", index))
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for (core, c) in reading.cores.iter().enumerate() {
                ui.label(format!("Core #{core}"));
                temp_bar(ui, c.temp_c, reading.tj_max_c);
                ui.weak(format!("{:>3}% load", c.load_pct));
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
