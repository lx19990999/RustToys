use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use std::io::{Read, Write};
use base64::Engine;

#[derive(Default)]
pub struct GZip {
    input: String,
    output: String,
    error: String,
    compress: bool,
}

impl Tool for GZip {
    fn name(&self) -> &str { "GZip Compress / Decompress" }
    fn description(&self) -> &str { "Compress or decompress text using GZip" }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.compress, true, "Compress");
            ui.radio_value(&mut self.compress, false, "Decompress");
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_mode = self.compress;

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
                                    self.input = if self.compress {
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
                    .id_salt("gzip_input_scroll")
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
                let out_label = if self.compress { "Compressed (Base64):" } else { "Decompressed text:" };
                ui.label(out_label);

                egui::ScrollArea::vertical()
                    .id_salt("gzip_output_scroll")
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
        if self.input != prev_input || self.compress != prev_mode {
            self.convert();
        }
    }
}

impl GZip {
    fn convert(&mut self) {
        self.error.clear();
        if self.input.trim().is_empty() {
            self.output.clear();
            return;
        }

        if self.compress {
            match self.compress_gzip(self.input.as_bytes()) {
                Ok(bytes) => {
                    self.output = base64::engine::general_purpose::STANDARD.encode(&bytes);
                }
                Err(e) => {
                    self.output.clear();
                    self.error = format!("Compress error: {}", e);
                }
            }
        } else {
            let cleaned: String = self.input.chars().filter(|c| !c.is_whitespace()).collect();
            match base64::engine::general_purpose::STANDARD.decode(cleaned.as_bytes()) {
                Ok(bytes) => match self.decompress_gzip(&bytes) {
                    Ok(decompressed) => {
                        self.output = String::from_utf8_lossy(&decompressed).to_string();
                    }
                    Err(e) => {
                        self.output.clear();
                        self.error = format!("Decompress error: {}", e);
                    }
                },
                Err(e) => {
                    self.output.clear();
                    self.error = format!("Base64 decode error: {}", e);
                }
            }
        }
    }

    fn compress_gzip(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data)?;
        encoder.finish()
    }

    fn decompress_gzip(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut result = Vec::new();
        decoder.read_to_end(&mut result)?;
        Ok(result)
    }
}
