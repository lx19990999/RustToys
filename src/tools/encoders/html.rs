use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};


pub struct HtmlEncoder {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
    pending_file: Pending<String>,
}

impl Default for HtmlEncoder {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encode_mode: false,
            pending_file: Pending::default(),
        }
    }
}


impl Tool for HtmlEncoder {
    fn name(&self) -> &str { "HTML Text Encoder / Decoder" }
    fn description(&self) -> &str { "Encode or decode HTML entities" }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encode_mode, true, "Encode");
            ui.radio_value(&mut self.encode_mode, false, "Decode");
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_mode = self.encode_mode;

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
                    open_file_async(&mut self.pending_file, "Open text file", "Text", &["txt"]);
                    }
                    if ui.button("Clear").clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label("Input:");

                egui::ScrollArea::vertical()
                    .id_salt("html_input_scroll")
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
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "output.txt") {
                            let _ = std::fs::write(path, &self.output);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label("Output:");

                egui::ScrollArea::vertical()
                    .id_salt("html_output_scroll")
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

        // Auto-convert
        if self.input != prev_input || self.encode_mode != prev_mode {
            self.convert();
        }
    }
}

impl HtmlEncoder {
    fn convert(&mut self) {
        self.error.clear();
        if self.input.is_empty() {
            self.output.clear();
            return;
        }

        if self.encode_mode {
            self.output = html_escape::encode_text(&self.input).to_string();
        } else {
            self.output = html_escape::decode_html_entities(&self.input).to_string();
        }
    }
}
