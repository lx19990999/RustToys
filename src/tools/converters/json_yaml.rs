use eframe::egui;
use crate::tool::{Tool, ToolCategory};
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
    fn name(&self) -> &str { "JSON <> YAML Converter" }
    fn description(&self) -> &str { "Convert between JSON and YAML formats" }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.to_yaml, true, "JSON → YAML");
            ui.radio_value(&mut self.to_yaml, false, "YAML → JSON");
            ui.separator();
            ui.label("Indent:");
            ui.add(egui::DragValue::new(&mut self.indent).range(0..=8).speed(1));
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_to_yaml = self.to_yaml;
        let prev_indent = self.indent;

        ui.columns(2, |cols| {
            // Left: Input panel
            cols[0].vertical(|ui| {
                let input_label = if self.to_yaml { "Input JSON:" } else { "Input YAML:" };

                ui.horizontal(|ui| {
                    if ui.button("Paste").clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = format!("Clipboard error: {}", e),
                        }
                    }
                    if ui.button("Open File...").clicked() {
                        open_file_async(&mut self.pending_file, "Open file", "Data", &["json", "yaml", "yml"]);
                    }
                    if ui.button("Clear").clicked() {
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

                let output_label = if self.to_yaml { "Output YAML:" } else { "Output JSON:" };

                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.output.is_empty() {
                        let ext = if self.to_yaml { "yaml" } else { "json" };
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", &ext.to_uppercase(), &[ext], &format!("output.{}", ext)) {
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
                        Err(e) => self.error = format!("YAML error: {}", e),
                    }
                }
                Err(e) => self.error = format!("JSON parse error: {}", e),
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
                        Err(e) => self.error = format!("JSON error: {}", e),
                    }
                }
                Err(e) => self.error = format!("YAML parse error: {}", e),
            }
        }
    }
}
