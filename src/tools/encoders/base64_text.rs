use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use base64::Engine;

#[derive(Default)]
pub struct Base64Text {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
}

impl Tool for Base64Text {
    fn name(&self) -> &str { "Base64 Text Encoder / Decoder" }
    fn description(&self) -> &str { "Encode or decode text using Base64" }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
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
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open file")
                            .add_filter("All files", &["*"])
                            .pick_file()
                        {
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    self.input = if self.encode_mode {
                                        String::from_utf8_lossy(&bytes).to_string()
                                    } else {
                                        String::from_utf8_lossy(&bytes).to_string()
                                    };
                                }
                                Err(e) => self.error = format!("File read error: {}", e),
                            }
                        }
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
                    .id_salt("b64_input_scroll")
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
                    .id_salt("b64_output_scroll")
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

impl Base64Text {
    fn convert(&mut self) {
        self.error.clear();
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }

        if self.encode_mode {
            self.output = base64::engine::general_purpose::STANDARD
                .encode(self.input.as_bytes());
        } else {
            match base64::engine::general_purpose::STANDARD
                .decode(self.input.trim().as_bytes())
            {
                Ok(bytes) => self.output = String::from_utf8_lossy(&bytes).to_string(),
                Err(e) => {
                    self.output.clear();
                    self.error = format!("Decode error: {}", e);
                }
            }
        }
    }
}
