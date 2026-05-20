use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use serde::Serialize;
use serde_json::Value;

pub struct JsonFormatter {
    input: String,
    output: String,
    error: String,
    indent: usize,
    sort_keys: bool,
    minify: bool,
    pending_file: Pending<String>,
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            indent: 2,
            sort_keys: false,
            minify: false,
            pending_file: Pending::default(),
        }
    }
}

impl Tool for JsonFormatter {
    fn name(&self) -> &str { "JSON Formatter" }
    fn description(&self) -> &str { "Format, minify, and validate JSON" }
    fn category(&self) -> ToolCategory { ToolCategory::Formatters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }
        let prev_input = self.input.clone();
        let prev_indent = self.indent;
        let prev_sort = self.sort_keys;
        let prev_minify = self.minify;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open JSON file", "JSON", &["json"]);
                    }
                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label("Indent:");
                    ui.add(egui::DragValue::new(&mut self.indent).range(0..=8).speed(1));
                    ui.checkbox(&mut self.minify, "Minify");
                    ui.checkbox(&mut self.sort_keys, "Sort keys");
                });
                ui.add_space(2.0);
                ui.label("Input JSON:");

                egui::ScrollArea::vertical()
                    .id_salt("json_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "JSON", &["json"], "formatted.json") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label("Output:");

                egui::ScrollArea::vertical()
                    .id_salt("json_output_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.output)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });

        // Auto-format
        if self.input != prev_input || self.indent != prev_indent
            || self.sort_keys != prev_sort || self.minify != prev_minify
        {
            self.format();
        }
    }
}

impl JsonFormatter {
    fn format(&mut self) {
        self.error.clear();
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }

        match serde_json::from_str::<Value>(&self.input) {
            Ok(val) => {
                let val = if self.sort_keys { sort_json_keys(&val) } else { val };

                if self.minify {
                    self.output = serde_json::to_string(&val).unwrap();
                } else if self.indent == 0 {
                    self.output = serde_json::to_string(&val).unwrap();
                } else {
                    let indent_str = " ".repeat(self.indent);
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent_str.as_bytes());
                    let mut buf = Vec::new();
                    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                    val.serialize(&mut ser).unwrap();
                    self.output = String::from_utf8(buf).unwrap();
                }
            }
            Err(e) => {
                self.output.clear();
                self.error = format!("JSON parse error: {}", e);
            }
        }
    }
}

fn sort_json_keys(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(k, _)| k.to_string());
            let sorted: serde_json::Map<String, Value> = entries
                .into_iter()
                .map(|(k, v)| (k.clone(), sort_json_keys(v)))
                .collect();
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_keys).collect()),
        other => other.clone(),
    }
}
