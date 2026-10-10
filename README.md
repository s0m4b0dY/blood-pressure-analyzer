# Blood pressure & headache diary

A small, **read-only**, native Rust app for examining a blood-pressure diary from TOML. It uses `egui`/`eframe` (Rust's immediate-mode GUI, **not** the C++ Dear ImGui binding) and `egui_plot`.

## Run

Use **Rust 1.95+** (required by egui 0.36).

```bash
cargo run --release
```

To open a different file:

```bash
cargo run --release -- /path/to/my-measurements.toml
```

By default, the project opens `data/input.toml`. Run these commands from the project root. After editing the TOML, press **Reload TOML**; invalid changes show an error and leave the last valid data visible. The app does not upload data anywhere and never modifies the source file.

Run tests:

```bash
cargo test
```

On Debian/Ubuntu, if native desktop dependencies are missing when compiling, install (package names may differ on your distribution):

```bash
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

## Supported TOML input

The original `data/input.toml` is included **unchanged**.

```toml
[[blood_pressure]]
date = "10.10.2026"
time = "8:00"  # also accepts "08:00"
upper_pressure = 100
lower_pressure = 70
pulse = 101  # optional
comment = "A little headache"  # optional
```

You can **also record symptoms independently of blood-pressure measurements**, useful if a headache starts between readings:

```toml
[[symptom]]
date = "11.10.2026"
time = "15:30"
description = "Headache started; temples; sensitivity to light"
severity = 4  # optional, 0–10

[[symptom]]
date = "11.10.2026"
time = "16:45"
description = "Pain resolved"
severity = 0
```

These second examples are **not** added to your actual data; they show a possible extension for future measurements. There is no need to repeat an observation in both `comment` and `[[symptom]]`.

## What is shown

- Blood pressure: systolic and diastolic lines, with diamonds marking comments attached to measurements.
- Pulse: individual dots (a missing pulse is **not** plotted as zero).
- Symptom timeline: original comments plus optional independent symptom entries, ordered by timestamp.
- Full original-measurement table (with a filter for annotated measurements).
- Counts, source filename, time span, and a manual reload button.

Graphs use **actual elapsed time**, not equally spaced readings. They can be zoomed and panned. Dates/times are handled as entered, **without guessing a timezone**. This is a visualization, not an assessment of possible medical causes.

## Source files

- `src/model.rs`: TOML parsing (`serde`, `toml`), input validation, sorted records (`chrono`), reusable `MeasurementFile` model, and tests.
- `src/ui.rs`: charts, tables, reload logic (`egui_plot`, `eframe`).
- `src/main.rs`: command-line file path and native app entry point.

This is intentionally a small project: no database, background workers, networking, or write-back into your doctor's diary.
