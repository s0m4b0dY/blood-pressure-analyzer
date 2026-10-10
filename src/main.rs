mod model;
mod ui;

use std::{env, path::PathBuf};
use eframe::egui;

fn main() -> eframe::Result {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/input.toml"));

    let file = match model::MeasurementFile::load(&path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Unable to load diary: {error:#}");
            std::process::exit(1);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1050.0, 830.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Blood pressure diary",
        options,
        Box::new(move |_cc| Ok(Box::new(ui::Viewer::new(file)))),
    )
}
