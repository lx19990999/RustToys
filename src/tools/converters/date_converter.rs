use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use chrono::{TimeZone, Utc, Local, NaiveDateTime, Datelike, Timelike};

const TIMEZONES: &[(&str, &str)] = &[
    ("UTC", "UTC"),
    ("Local", "Local"),
    ("US/Eastern", "America/New_York"),
    ("US/Central", "America/Chicago"),
    ("US/Mountain", "America/Denver"),
    ("US/Pacific", "America/Los_Angeles"),
    ("Europe/London", "Europe/London"),
    ("Europe/Paris", "Europe/Paris"),
    ("Europe/Berlin", "Europe/Berlin"),
    ("Asia/Tokyo", "Asia/Tokyo"),
    ("Asia/Shanghai", "Asia/Shanghai"),
    ("Asia/Kolkata", "Asia/Kolkata"),
    ("Australia/Sydney", "Australia/Sydney"),
];

pub struct DateConverter {
    input: String,
    format: String,
    output: String,
    tz_index: usize,
}

impl Default for DateConverter {
    fn default() -> Self {
        Self {
            input: String::new(),
            format: "%Y-%m-%d %H:%M:%S %Z".to_string(),
            output: String::new(),
            tz_index: 0,
        }
    }
}

impl Tool for DateConverter {
    fn name(&self) -> &str { "Date Converter" }
    fn description(&self) -> &str { "Convert dates between formats, timestamps, and timezones" }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Input (timestamp or date string):");
        ui.text_edit_singleline(&mut self.input);
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Output format:");
            ui.text_edit_singleline(&mut self.format);
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Target timezone:");
            egui::ComboBox::from_id_salt("date_tz")
                .selected_text(TIMEZONES[self.tz_index].0)
                .show_ui(ui, |ui| {
                    for (i, (name, _)) in TIMEZONES.iter().enumerate() {
                        ui.selectable_value(&mut self.tz_index, i, *name);
                    }
                });
        });
        ui.add_space(4.0);

        if ui.button("Convert").clicked() {
            let fmt = if self.format.is_empty() { "%Y-%m-%d %H:%M:%S %Z" } else { &self.format };
            self.output = match self.input.trim().parse::<i64>() {
                Ok(ts) => {
                    let dt = if ts > 1_000_000_000_000 {
                        Utc.timestamp_millis_opt(ts).single()
                    } else {
                        Utc.timestamp_opt(ts, 0).single()
                    };
                    match dt {
                        Some(dt) => self.format_output(dt, fmt),
                        None => "Invalid timestamp".to_string(),
                    }
                }
                Err(_) => {
                    let formats = [
                        "%Y-%m-%d %H:%M:%S",
                        "%Y-%m-%dT%H:%M:%S",
                        "%Y-%m-%dT%H:%M:%SZ",
                        "%Y-%m-%dT%H:%M:%S%:z",
                        "%Y-%m-%d",
                        "%d/%m/%Y",
                        "%m/%d/%Y",
                        "%Y/%m/%d",
                    ];
                    let mut found = None;
                    for f in &formats {
                        if let Ok(ndt) = NaiveDateTime::parse_from_str(self.input.trim(), f) {
                            found = Some(Utc.from_utc_datetime(&ndt));
                            break;
                        }
                    }
                    if found.is_none() {
                        for f in &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y"] {
                            if let Ok(nd) = chrono::NaiveDate::parse_from_str(self.input.trim(), f) {
                                found = Some(Utc.from_utc_datetime(&nd.and_hms_opt(0, 0, 0).unwrap()));
                                break;
                            }
                        }
                    }
                    match found {
                        Some(dt) => self.format_output(dt, fmt),
                        None => "Could not parse date. Try a timestamp or YYYY-MM-DD format.".to_string(),
                    }
                }
            };
        }

        ui.add_space(8.0);
        ui.label("Output:");
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button("Save As...").clicked() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "date_output.txt") {
                        let _ = std::fs::write(path, &self.output);
                    }
                }
            });
        }
    }
}

impl DateConverter {
    fn format_output(&self, utc_dt: chrono::DateTime<Utc>, fmt: &str) -> String {
        let local = utc_dt.with_timezone(&Local);
        let mut out = String::new();

        out.push_str(&format!("UTC:   {}\n", utc_dt.format(fmt)));
        out.push_str(&format!("Local: {}\n", local.format(fmt)));
        out.push_str(&format!("Timestamp (s):  {}\n", utc_dt.timestamp()));
        out.push_str(&format!("Timestamp (ms): {}\n", utc_dt.timestamp_millis()));

        // DST info for local timezone
        let offset = local.offset();
        out.push_str(&format!("Local UTC offset: {}\n", offset));

        // Day of year, ISO week
        out.push_str(&format!("Day of year: {}\n", utc_dt.ordinal()));
        out.push_str(&format!("ISO week:    {}\n", utc_dt.iso_week().week()));
        out.push_str(&format!("Day of week: {}\n",
            ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                [utc_dt.weekday().num_days_from_monday() as usize]
        ));

        // Common format conversions
        out.push_str("\n--- Common Formats ---\n");
        out.push_str(&format!("ISO 8601:      {}\n", utc_dt.format("%Y-%m-%dT%H:%M:%SZ")));
        out.push_str(&format!("RFC 2822:      {}\n", utc_dt.format("%a, %d %b %Y %H:%M:%S +0000")));
        out.push_str(&format!("Human readable: {}\n", utc_dt.format("%B %d, %Y %I:%M %p")));

        out
    }
}
