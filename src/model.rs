use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;

// Only these two structs know what the TOML file looks like.
#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    blood_pressure: Vec<RawPressure>,
    #[serde(default)]
    symptom: Vec<RawSymptom>,
}

#[derive(Deserialize)]
struct RawPressure {
    date: String,
    time: String,
    upper_pressure: u16,
    lower_pressure: u16,
    pulse: Option<u16>,
    comment: Option<String>,
}

// Optional: record a symptom at a time when no pressure was measured.
#[derive(Deserialize)]
struct RawSymptom {
    date: String,
    time: String,
    description: String,
    severity: Option<u8>, // 0 to 10, if known
}

#[derive(Debug, Clone)]
pub struct PressureReading {
    pub at: NaiveDateTime,
    pub systolic: u16,
    pub diastolic: u16,
    pub pulse: Option<u16>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Symptom {
    pub at: NaiveDateTime,
    pub description: String,
    pub severity: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct Note<'a> {
    pub at: NaiveDateTime,
    pub text: &'a str,
    pub severity: Option<u8>,
}

#[derive(Debug)]
pub struct MeasurementFile {
    pub path: PathBuf,
    pub readings: Vec<PressureReading>,
    pub symptoms: Vec<Symptom>,
}

impl MeasurementFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        Self::parse(path, &contents)
    }

    pub fn parse(path: impl AsRef<Path>, contents: &str) -> Result<Self> {
        let raw: RawFile = toml::from_str(contents).context("Invalid TOML")?;
        ensure!(
            !raw.blood_pressure.is_empty() || !raw.symptom.is_empty(),
            "The file has no [[blood_pressure]] or [[symptom]] entries"
        );

        let mut readings = Vec::with_capacity(raw.blood_pressure.len());
        for (i, item) in raw.blood_pressure.into_iter().enumerate() {
            let at = parse_datetime(&item.date, &item.time)
                .with_context(|| format!("blood_pressure entry #{}", i + 1))?;
            ensure!(
                item.systolic_is_valid(),
                "blood_pressure entry #{}: systolic must exceed diastolic (both must be positive)",
                i + 1
            );
            if item.pulse == Some(0) {
                bail!("blood_pressure entry #{}: pulse cannot be zero", i + 1);
            }
            readings.push(PressureReading {
                at,
                systolic: item.upper_pressure,
                diastolic: item.lower_pressure,
                pulse: item.pulse,
                comment: item.comment,
            });
        }

        let mut symptoms = Vec::with_capacity(raw.symptom.len());
        for (i, item) in raw.symptom.into_iter().enumerate() {
            let at = parse_datetime(&item.date, &item.time)
                .with_context(|| format!("symptom entry #{}", i + 1))?;
            ensure!(
                !item.description.trim().is_empty(),
                "symptom entry #{}: description cannot be empty",
                i + 1
            );
            ensure!(
                item.severity.is_none_or(|severity| severity <= 10),
                "symptom entry #{}: severity must be between 0 and 10",
                i + 1
            );
            symptoms.push(Symptom {
                at,
                description: item.description,
                severity: item.severity,
            });
        }

        readings.sort_by_key(|reading| reading.at);
        symptoms.sort_by_key(|symptom| symptom.at);

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            readings,
            symptoms,
        })
    }

    // Both recorded comments and separate symptom entries appear on the timeline.
    pub fn notes(&self) -> Vec<Note<'_>> {
        let mut notes: Vec<_> = self
            .readings
            .iter()
            .filter_map(|reading| {
                reading.comment.as_deref().filter(|text| !text.trim().is_empty()).map(|text| Note {
                    at: reading.at,
                    text,
                    severity: None,
                })
            })
            .chain(self.symptoms.iter().map(|symptom| Note {
                at: symptom.at,
                text: symptom.description.as_str(),
                severity: symptom.severity,
            }))
            .collect();
        notes.sort_by_key(|note| note.at);
        notes
    }

    pub fn start(&self) -> NaiveDateTime {
        self.readings
            .first()
            .map(|reading| reading.at)
            .into_iter()
            .chain(self.symptoms.first().map(|symptom| symptom.at))
            .min()
            .expect("A MeasurementFile always has at least one entry")
    }

    pub fn end(&self) -> NaiveDateTime {
        self.readings
            .last()
            .map(|reading| reading.at)
            .into_iter()
            .chain(self.symptoms.last().map(|symptom| symptom.at))
            .max()
            .expect("A MeasurementFile always has at least one entry")
    }
}

impl RawPressure {
    fn systolic_is_valid(&self) -> bool {
        self.lower_pressure > 0 && self.upper_pressure > self.lower_pressure
    }
}

fn parse_datetime(date: &str, time: &str) -> Result<NaiveDateTime> {
    let date = NaiveDate::parse_from_str(date, "%d.%m.%Y")
        .with_context(|| format!("Invalid date '{date}' (expected DD.MM.YYYY)"))?;

    // Accept both "8:00" and "08:00", without inventing a timezone.
    let (hour, minute) = time
        .split_once(':')
        .with_context(|| format!("Invalid time '{time}' (expected H:MM)"))?;
    let hour: u32 = hour.parse().with_context(|| format!("Invalid hour in '{time}'"))?;
    let minute: u32 = minute.parse().with_context(|| format!("Invalid minute in '{time}'"))?;
    let time = NaiveTime::from_hms_opt(hour, minute, 0)
        .with_context(|| format!("Invalid clock time '{time}'"))?;
    Ok(date.and_time(time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supplied_data_without_inventing_pulse() {
        let file = MeasurementFile::parse("test.toml", include_str!("../data/input.toml")).unwrap();
        assert_eq!(file.readings.len(), 11);
        assert_eq!(file.readings.iter().filter(|r| r.pulse.is_none()).count(), 1);
        assert_eq!(file.notes().len(), 2);
        assert_eq!(file.readings[0].at.format("%d.%m.%Y %H:%M").to_string(), "06.10.2026 08:00");
    }

    #[test]
    fn sorts_readings_and_accepts_single_digit_hours() {
        let source = r#"
            [[blood_pressure]]
            date = "07.10.2026"
            time = "20:00"
            upper_pressure = 110
            lower_pressure = 66

            [[blood_pressure]]
            date = "07.10.2026"
            time = "8:00"
            upper_pressure = 112
            lower_pressure = 65
            pulse = 78
        "#;
        let file = MeasurementFile::parse("test", source).unwrap();
        assert_eq!(file.readings[0].systolic, 112);
        assert!(file.readings[1].pulse.is_none());
    }

    #[test]
    fn separate_symptoms_do_not_need_pressure_readings() {
        let source = r#"
            [[symptom]]
            date = "10.10.2026"
            time = "13:20"
            description = "Headache started"
            severity = 3
        "#;
        let file = MeasurementFile::parse("test", source).unwrap();
        assert!(file.readings.is_empty());
        assert_eq!(file.notes()[0].severity, Some(3));
    }

    #[test]
    fn rejects_invalid_datetime_and_severity() {
        let bad_date = r#"
            [[blood_pressure]]
            date = "31.02.2026"
            time = "08:00"
            upper_pressure = 110
            lower_pressure = 70
        "#;
        assert!(MeasurementFile::parse("test", bad_date).is_err());

        let bad_severity = r#"
            [[symptom]]
            date = "10.10.2026"
            time = "13:20"
            description = "Headache"
            severity = 11
        "#;
        assert!(MeasurementFile::parse("test", bad_severity).is_err());
    }
}
