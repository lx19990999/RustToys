use eframe::egui;
use crate::tool::{Tool, ToolCategory};

#[derive(Default)]
pub struct CronParser {
    input: String,
    output: String,
    description: String,
    count: usize,
    date_format: String,
    include_seconds: bool,
}

impl Tool for CronParser {
    fn name(&self) -> &str { "Cron Parser" }
    fn description(&self) -> &str { "Parse cron expressions and show upcoming execution times" }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if self.count == 0 {
            self.count = 5;
            self.date_format = "%Y-%m-%d %H:%M:%S".to_string();
        }

        ui.label("Cron Expression:");
        ui.text_edit_singleline(&mut self.input);
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.include_seconds, "Include seconds in expression");
            ui.separator();
            ui.label("Count:");
            ui.add(egui::DragValue::new(&mut self.count).range(1..=50).speed(1));
        });
        ui.add_space(4.0);

        ui.label("Output date time format (chrono strftime):");
        ui.text_edit_singleline(&mut self.date_format);
        ui.add_space(4.0);

        if ui.button("Parse").clicked() {
            self.description.clear();
            self.output.clear();

            let expr = self.input.trim();
            let cron_expr = if self.include_seconds {
                expr.to_string()
            } else {
                // cron crate expects 6-field format with seconds
                format!("0 {}", expr)
            };

            match cron_expr.parse::<cron::Schedule>() {
                Ok(schedule) => {
                    // Generate human-readable description
                    self.description = describe_cron(expr, self.include_seconds);

                    let times: Vec<String> = schedule
                        .upcoming(chrono::Local)
                        .take(self.count)
                        .map(|dt| dt.format(&self.date_format).to_string())
                        .collect();
                    if times.is_empty() {
                        self.output = "No upcoming times found".to_string();
                    } else {
                        self.output = times.join("\n");
                    }
                }
                Err(e) => {
                    self.output = format!("Cron expression is not valid: {}", e);
                }
            }
        }

        if !self.description.is_empty() {
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Cron description").strong());
                ui.label(&self.description);
            });
            ui.add_space(4.0);
        }

        ui.label("Next scheduled dates:");
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button("Save As...").clicked() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "cron_schedule.txt") {
                        let _ = std::fs::write(path, &self.output);
                    }
                }
            });
        }
    }
}

fn describe_cron(expr: &str, has_seconds: bool) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    let (sec, min, hour, dom, month, dow) = if has_seconds && fields.len() == 6 {
        (fields[0], fields[1], fields[2], fields[3], fields[4], fields[5])
    } else if !has_seconds && fields.len() == 5 {
        ("*", fields[0], fields[1], fields[2], fields[3], fields[4])
    } else {
        return format!("Unable to parse (expected {} fields)", if has_seconds { 6 } else { 5 });
    };

    let mut parts = Vec::new();

    // Seconds
    if has_seconds && sec != "*" {
        parts.push(format!("at second {}", describe_field(sec, "second")));
    }

    // Minutes
    if min != "*" {
        parts.push(format!("at minute {}", describe_field(min, "minute")));
    }

    // Hours
    if hour != "*" {
        parts.push(format!("at hour {}", describe_field(hour, "hour")));
    }

    // Day of month
    if dom != "*" {
        parts.push(format!("on day {} of the month", describe_field(dom, "day")));
    }

    // Month
    if month != "*" {
        let month_names = ["", "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"];
        parts.push(format!("in {}", describe_named_field(month, &month_names)));
    }

    // Day of week
    if dow != "*" {
        let dow_names = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        parts.push(format!("on {}", describe_named_field(dow, &dow_names)));
    }

    if parts.is_empty() {
        return "Every second (always)".to_string();
    }

    format!("Runs {}", parts.join(", "))
}

fn describe_field(field: &str, unit: &str) -> String {
    if field.contains(',') {
        let items: Vec<String> = field.split(',').map(|s| s.trim().to_string()).collect();
        return items.join(", ");
    }
    if let Some(stripped) = field.strip_prefix("*/") {
        return format!("every {} {}s", stripped, unit);
    }
    if field.contains('-') {
        return field.to_string();
    }
    field.to_string()
}

fn describe_named_field(field: &str, names: &[&str]) -> String {
    if field.contains(',') {
        let items: Vec<String> = field.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if let Ok(n) = s.parse::<usize>() {
                    names.get(n).map(|n| n.to_string()).or(Some(s.to_string()))
                } else {
                    Some(s.to_string())
                }
            })
            .collect();
        return items.join(", ");
    }
    if let Some(stripped) = field.strip_prefix("*/") {
        return format!("every {}th", stripped);
    }
    if let Ok(n) = field.parse::<usize>() {
        if let Some(name) = names.get(n) {
            return name.to_string();
        }
    }
    field.to_string()
}
