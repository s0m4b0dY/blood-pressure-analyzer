use chrono::{Duration, NaiveDateTime};
use eframe::egui::{self, Color32, TextBuffer};
use egui_plot::{HoverPosition, Legend, Line, MarkerShape, Plot, Points};

use crate::model::MeasurementFile;

const SYSTOLIC_COLOR: Color32 = Color32::from_rgb(225, 100, 100);
const DIASTOLIC_COLOR: Color32 = Color32::from_rgb(90, 160, 240);
const PULSE_COLOR: Color32 = Color32::from_rgb(70, 200, 160);
const NOTE_COLOR: Color32 = Color32::from_rgb(235, 180, 75);

pub struct Viewer {
    file: MeasurementFile,
    load_error: Option<String>,
    notes_only: bool,
}

impl Viewer {
    pub fn new(file: MeasurementFile) -> Self {
        Self {
            file,
            load_error: None,
            notes_only: false,
        }
    }

    fn reload(&mut self) {
        match MeasurementFile::load(&self.file.path) {
            Ok(file) => {
                self.file = file;
                self.load_error = None;
            }
            Err(error) => {
                // Keep displaying the previous valid file instead of discarding it.
                self.load_error = Some(format!("{error:#}"));
            }
        }
    }

    fn summary(&self, ui: &mut egui::Ui) {
        let missing_pulse = self.file.readings.iter().filter(|r| r.pulse.is_none()).count();
        ui.label(format!(
            "{} to {}  |  {} pressure readings  |  {} notes  |  {} missing pulse values",
            self.file.start().format("%d.%m.%Y"),
            self.file.end().format("%d.%m.%Y"),
            self.file.readings.len(),
            self.file.notes().len(),
            missing_pulse
        ));
    }

    fn pressure_graph(&self, ui: &mut egui::Ui) {
        if self.file.readings.is_empty() {
            ui.label("No blood pressure readings in this file.");
            return;
        }
        let start = self.file.start();
        let systolic: Vec<_> = self.file.readings.iter()
            .map(|r| [days_since(start, r.at), f64::from(r.systolic)])
            .collect();
        let diastolic: Vec<_> = self.file.readings.iter()
            .map(|r| [days_since(start, r.at), f64::from(r.diastolic)])
            .collect();
        let marked: Vec<_> = self.file.readings.iter()
            .filter(|r| r.comment.as_ref().is_some_and(|s| !s.trim().is_empty()))
            .map(|r| (days_since(start, r.at), r.comment.as_ref().unwrap().clone()))
            .collect();
        let note_points: Vec<_> = self.file.readings.iter()
            .filter(|r| r.comment.as_ref().is_some_and(|s| !s.trim().is_empty()))
            .map(|r| [days_since(start, r.at), f64::from(r.systolic)])
            .collect();

        Plot::new("pressure")
            .height(280.0)
            .legend(Legend::default())
            .x_axis_label("Date / time")
            .y_axis_label("mmHg")
            .x_axis_formatter(move |mark, _| axis_date(start, mark.value))
            .label_formatter(move |hover| hover_label(hover, start, &marked))
            .show(ui, |plot| {
                plot.line(Line::new("Systolic", systolic.clone()).id("systolic-line").color(SYSTOLIC_COLOR));
                plot.points(Points::new("Systolic", systolic).id("systolic-dots").radius(3.0).color(SYSTOLIC_COLOR));
                plot.line(Line::new("Diastolic", diastolic.clone()).id("diastolic-line").color(DIASTOLIC_COLOR));
                plot.points(Points::new("Diastolic", diastolic).id("diastolic-dots").radius(3.0).color(DIASTOLIC_COLOR));
                if !note_points.is_empty() {
                    plot.points(Points::new("Comment", note_points)
                        .shape(MarkerShape::Diamond)
                        .radius(7.0)
                        .color(NOTE_COLOR));
                }
            });
        ui.small("Diamonds indicate measurements with a written comment. Drag to pan, scroll to zoom.");
    }

    fn pulse_graph(&self, ui: &mut egui::Ui) {
        let start = self.file.start();
        let pulse: Vec<_> = self.file.readings.iter()
            .filter_map(|r| r.pulse.map(|value| [days_since(start, r.at), f64::from(value)]))
            .collect();

        if pulse.is_empty() {
            ui.label("No pulse measurements in this file.");
            return;
        }
        Plot::new("pulse")
            .height(190.0)
            .x_axis_label("Date / time")
            .y_axis_label("bpm")
            .x_axis_formatter(move |mark, _| axis_date(start, mark.value))
            .label_formatter(move |hover| hover_label(hover, start, &[]))
            .show(ui, |plot| {
                // Points instead of a connecting line: no invented value between readings.
                plot.points(Points::new("Pulse", pulse).color(PULSE_COLOR).radius(5.0));
            });
        ui.small("Pulse is shown as individual measurements. Missing values are left empty.");
    }

    fn notes(&self, ui: &mut egui::Ui) {
        let notes = self.file.notes();
        if notes.is_empty() {
            ui.label("No symptom notes recorded yet.");
            return;
        }
        egui::Grid::new("notes_grid").striped(true).num_columns(3).show(ui, |ui| {
            ui.strong("Date / time");
            ui.strong("Severity");
            ui.strong("Comment");
            ui.end_row();
            for note in notes {
                ui.monospace(note.at.format("%d.%m.%Y %H:%M").to_string());
                ui.label(note.severity.map_or_else(|| "—".to_owned(), |x| format!("{x}/10")));
                ui.label(note.text);
                ui.end_row();
            }
        });
    }

    fn readings(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.notes_only, "Show only measurements with comments");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("readings_grid").striped(true).num_columns(4).show(ui, |ui| {
                ui.strong("Date / time");
                ui.strong("Blood pressure (mmHg)");
                ui.strong("Pulse (bpm)");
                ui.strong("Original comment");
                ui.end_row();
                for r in &self.file.readings {
                    if self.notes_only && r.comment.as_ref().is_none_or(|s| s.trim().is_empty()) {
                        continue;
                    }
                    ui.monospace(r.at.format("%d.%m.%Y %H:%M").to_string());
                    ui.label(format!("{} / {}", r.systolic, r.diastolic));
                    ui.label(r.pulse.map_or_else(|| "—".to_owned(), |v| v.to_string()));
                    ui.label(r.comment.as_deref().unwrap_or("—"));
                    ui.end_row();
                }
            });
        });
    }
}

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Blood pressure & headache diary");
            ui.horizontal_wrapped(|ui| {
                ui.monospace(self.file.path.display().to_string());
                if ui.button("Reload TOML").clicked() {
                    self.reload();
                }
            });
            if let Some(error) = &self.load_error {
                ui.colored_label(Color32::LIGHT_RED, format!("Reload failed: {error}"));
            }
            ui.add_space(8.0);
            self.summary(ui);
            ui.separator();

            ui.heading("Blood pressure");
            self.pressure_graph(ui);
            ui.add_space(10.0);
            ui.heading("Pulse");
            self.pulse_graph(ui);
            ui.separator();

            ui.heading("Symptom timeline");
            self.notes(ui);
            ui.add_space(12.0);
            ui.heading("All pressure readings");
            self.readings(ui);
            ui.add_space(10.0);
            ui.small("A visualization of recorded observations, not a medical diagnosis. Times are local clock times as entered in the TOML file.");
        });
    }
}

// The x-coordinate is days since the first observation (not a fake evenly spaced index).
fn days_since(start: NaiveDateTime, at: NaiveDateTime) -> f64 {
    at.signed_duration_since(start).num_seconds() as f64 / 86_400.0
}

fn at_days(start: NaiveDateTime, days: f64) -> NaiveDateTime {
    start + Duration::seconds((days * 86_400.0).round() as i64)
}

fn axis_date(start: NaiveDateTime, value: f64) -> String {
    at_days(start, value).format("%d.%m %H:%M").to_string()
}

fn hover_label(hover: &HoverPosition<'_>, start: NaiveDateTime, notes: &[(f64, String)]) -> Option<String> {
    match hover {
        HoverPosition::NearDataPoint { plot_name, position, .. } if !plot_name.is_empty() => {
            let mut label = format!(
                "{} | {} | {:.0}",
                at_days(start, position.x).format("%d.%m.%Y %H:%M"),
                plot_name,
                position.y,
            );
            if let Some((_, text)) = notes.iter().find(|(x, _)| (*x - position.x).abs() < 0.1) {
                label.push_str(&format!("\n{text}"));
            }
            Some(label)
        }
        _ => None,
    }
}
