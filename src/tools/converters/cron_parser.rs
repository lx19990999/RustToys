use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, save_file_async};

pub struct CronParser {
    input: String,
    output: String,
    description: String,
    count: usize,
    date_format: String,
    include_seconds: bool,
    save_result: String,
    pending_file: Pending<String>,
}

impl Default for CronParser {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            description: String::new(),
            count: 0,
            date_format: String::new(),
            include_seconds: false,
            save_result: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl Tool for CronParser {
    fn name(&self) -> String { tr!("cron_name") }
    fn description(&self) -> String { tr!("cron_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            self.save_result = text;
        }

        if self.count == 0 {
            self.count = 5;
            self.date_format = "%Y-%m-%d %H:%M:%S".to_string();
        }

        ui.label(tr!("cron_expression_label"));
        ui.text_edit_singleline(&mut self.input);
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.include_seconds, tr!("cron_include_seconds"));
            ui.separator();
            ui.label(tr!("label_count"));
            ui.add(egui::DragValue::new(&mut self.count).range(1..=50).speed(1));
        });
        ui.add_space(4.0);

        ui.label(tr!("cron_output_format"));
        ui.text_edit_singleline(&mut self.date_format);
        ui.add_space(4.0);

        if ui.button(tr!("btn_parse")).clicked() {
            self.description.clear();
            self.output.clear();
            self.save_result.clear();

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
                        self.output = tr!("cron_no_times");
                    } else {
                        self.output = times.join("\n");
                    }
                }
                Err(e) => {
                    self.output = tr!("cron_error", e);
                }
            }
        }

        if !self.description.is_empty() {
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new(tr!("cron_description")).strong());
                ui.label(&self.description);
            });
            ui.add_space(4.0);
        }

        ui.label(tr!("cron_next_dates"));
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                if ui.button(tr!("btn_copy")).clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button(tr!("btn_save_as")).clicked() {
                    save_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("cron_save_default"), self.output.clone());
                }
            });
        }
        if !self.save_result.is_empty() {
            ui.colored_label(egui::Color32::GREEN, &self.save_result);
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
        return tr!("cron_unable_parse", if has_seconds { 6 } else { 5 });
    };

    let mut parts = Vec::new();

    // Seconds
    if has_seconds && sec != "*" {
        parts.push(tr!("cron_at_second", describe_field(sec, "second")));
    }

    // Minutes
    if min != "*" {
        parts.push(tr!("cron_at_minute", describe_field(min, "minute")));
    }

    // Hours
    if hour != "*" {
        parts.push(tr!("cron_at_hour", describe_field(hour, "hour")));
    }

    // Day of month
    if dom != "*" {
        parts.push(tr!("cron_on_day", describe_field(dom, "day")));
    }

    // Month
    if month != "*" {
        let month_names = vec![String::new(), tr!("cron_january"), tr!("cron_february"), tr!("cron_march"), tr!("cron_april"), tr!("cron_may"), tr!("cron_june"),
            tr!("cron_july"), tr!("cron_august"), tr!("cron_september"), tr!("cron_october"), tr!("cron_november"), tr!("cron_december")];
        parts.push(tr!("cron_in_month", describe_named_field(month, &month_names)));
    }

    // Day of week
    if dow != "*" {
        let dow_names = vec![tr!("cron_sunday"), tr!("cron_monday"), tr!("cron_tuesday"), tr!("cron_wednesday"), tr!("cron_thursday"), tr!("cron_friday"), tr!("cron_saturday")];
        parts.push(tr!("cron_on_dow", describe_named_field(dow, &dow_names)));
    }

    if parts.is_empty() {
        return tr!("cron_every_second");
    }

    tr!("cron_runs", parts.join(", "))
}

fn describe_field(field: &str, unit: &str) -> String {
    if field.contains(',') {
        let items: Vec<String> = field.split(',').map(|s| s.trim().to_string()).collect();
        return items.join(", ");
    }
    if let Some(stripped) = field.strip_prefix("*/") {
        return tr!("cron_every_n", stripped, unit);
    }
    if field.contains('-') {
        return field.to_string();
    }
    field.to_string()
}

fn describe_named_field(field: &str, names: &[String]) -> String {
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
        return tr!("cron_every_nth", stripped);
    }
    if let Ok(n) = field.parse::<usize>() {
        if let Some(name) = names.get(n) {
            return name.to_string();
        }
    }
    field.to_string()
}
