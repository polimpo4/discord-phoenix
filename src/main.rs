#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod config;
mod demo;
mod emoji;
mod gateway;
mod images;
mod markdown;
mod model;
mod theme;
mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([860.0, 560.0])
            .with_title("Discord Phoenix"),
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "Discord Phoenix",
        options,
        Box::new(|cc| Ok(Box::new(ui::App::new(cc)))),
    )
}
