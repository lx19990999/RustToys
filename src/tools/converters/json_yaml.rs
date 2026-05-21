use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};
use serde_json::Value;
use serde::Serialize;


pub struct JsonYaml {
    input: String,
    output: String,
    error: String,
    to_yaml: bool,
    indent: usize,
    pending_file: Pending<String>,
}

impl Default for JsonYaml {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            to_yaml: false,
            indent: 0,
            pending_file: Pending::default(),
        }
    }
}


impl Tool for JsonYaml {
    fn name(&self) -> String { tr!("jy_name") }
    fn description(&self) -> String { tr!("jy_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
            }
        }
        let label_jy_json_to_yaml = tr!("jy_json_to_yaml");
        let label_jy_yaml_to_json = tr!("jy_yaml_to_json");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.to_yaml, true, &label_jy_json_to_yaml);
            ui.radio_value(&mut self.to_yaml, false, &label_jy_yaml_to_json);
            ui.separator();
            ui.label(tr!("label_indent"));
            ui.add(egui::DragValue::new(&mut self.indent).range(0..=8).speed(1));
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_to_yaml = self.to_yaml;
        let prev_indent = self.indent;

        ui.columns(2, |cols| {
            // Left: Input panel
            cols[0].vertical(|ui| {
                let input_label = if self.to_yaml { tr!("jy_input_json") } else { tr!("jy_input_yaml") };

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["json", "yaml", "yml"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(input_label);

                egui::ScrollArea::vertical()
                    .id_salt("jy_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output panel
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                let output_label = if self.to_yaml { tr!("jy_output_yaml") } else { tr!("jy_output_json") };

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        let ext = if self.to_yaml { "yaml" } else { "json" };
                        if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("save_as_title"), &ext.to_uppercase(), &[ext], &format!("output.{}", ext)) {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(output_label);

                egui::ScrollArea::vertical()
                    .id_salt("jy_output_scroll")
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

        // Auto-convert when input, direction, or indent changes
        if self.input != prev_input || self.to_yaml != prev_to_yaml || self.indent != prev_indent {
            if !self.input.trim().is_empty() {
                self.convert();
            } else {
                self.output.clear();
                self.error.clear();
            }
        }
    }
}

impl JsonYaml {
    fn convert(&mut self) {
        self.error.clear();
        self.output.clear();

        if self.to_yaml {
            match serde_json::from_str::<Value>(&self.input) {
                Ok(val) => {
                    match serde_yaml::to_string(&val) {
                        Ok(yaml) => self.output = yaml,
                        Err(e) => self.error = tr!("jy_yaml_error", e),
                    }
                }
                Err(e) => self.error = tr!("jy_json_parse_error", e),
            }
        } else {
            match serde_yaml::from_str::<Value>(&self.input) {
                Ok(val) => {
                    let json = if self.indent == 0 {
                        serde_json::to_string(&val)
                    } else {
                        let indent = " ".repeat(self.indent);
                        let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
                        let mut buf = Vec::new();
                        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                        val.serialize(&mut ser).map(|_| String::from_utf8(buf).unwrap_or_default())
                    };
                    match json {
                        Ok(j) => self.output = j,
                        Err(e) => self.error = tr!("jy_json_error", e),
                    }
                }
                Err(e) => self.error = tr!("jy_yaml_parse_error", e),
            }
        }
    }
}
