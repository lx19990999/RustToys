use eframe::egui;
use crate::tr;
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
    fn name(&self) -> String { tr!("jf_name") }
    fn description(&self) -> String { tr!("jf_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Formatters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            let err_prefix = tr!("err_error_reading");
            if !text.starts_with(&err_prefix) {
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
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("jf_input_label"), "JSON", &["json"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label(tr!("label_indent"));
                    ui.add(egui::DragValue::new(&mut self.indent).range(0..=8).speed(1));
                    let label_minify = tr!("label_minify");
                    ui.checkbox(&mut self.minify, &label_minify);
                    let label_sort_keys = tr!("label_sort_keys");
                    ui.checkbox(&mut self.sort_keys, &label_sort_keys);
                });
                ui.add_space(2.0);
                ui.label(tr!("jf_input_label"));

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
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("save_as_title"), "JSON", &["json"], "formatted.json") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_output"));

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
                self.error = tr!("jy_json_parse_error", e);
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
