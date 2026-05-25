use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use crate::tools::io_layout;
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
    save_pending: Pending<String>,
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
            save_pending: Pending::default(),
        }
    }
}

impl Tool for JsonFormatter {
    fn name(&self) -> String { tr!("jf_name") }
    fn description(&self) -> String { tr!("jf_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Formatters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_indent = self.indent;
        let prev_sort = self.sort_keys;
        let prev_minify = self.minify;

        if let Some(text) = self.pending_file.poll() {
            let err_prefix = tr!("err_error_reading");
            if !text.starts_with(&err_prefix) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }

        let lbl_paste = tr!("btn_paste");
        let lbl_open_file = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_indent = tr!("label_indent");
        let lbl_minify = tr!("label_minify");
        let lbl_sort_keys = tr!("label_sort_keys");
        let lbl_input = tr!("jf_input_label");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_output = tr!("label_output");
        let err_clipboard = tr!("err_clipboard");

        let opt_h = io_layout::option_row_height(ui);
        io_layout::show_error(ui, &self.error);
        io_layout::two_column_io(ui, |ui, w, col| match col {
            io_layout::IoColumn::Left => {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_paste).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = err_clipboard.replace("{}", &e.to_string()),
                        }
                    }
                    if ui.button(&lbl_open_file).clicked() {
                        open_file_async(&mut self.pending_file, &lbl_input, "JSON", &["json"]);
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.horizontal(|ui| {
                    ui.label(&lbl_indent);
                    ui.add(egui::DragValue::new(&mut self.indent).range(0..=8).speed(1));
                    ui.checkbox(&mut self.minify, &lbl_minify);
                    ui.checkbox(&mut self.sort_keys, &lbl_sort_keys);
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.label(&lbl_input);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "json_input_scroll", &mut self.input);
            }
            io_layout::IoColumn::Right => {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                        save_file_async(
                            &mut self.save_pending,
                            &tr!("save_as_title"),
                            "JSON",
                            &["json"],
                            "formatted.json",
                            self.output.clone(),
                        );
                    }
                });
                io_layout::row_spacer(ui, opt_h + io_layout::ROW_GAP);
                ui.label(&lbl_output);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "json_output_scroll", &mut self.output);
            }
        });

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
