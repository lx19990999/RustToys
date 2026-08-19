use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, save_file_async};
use std::io::{Read, Write};
use base64::Engine;

pub struct GZip {
    input: String,
    output: String,
    error: String,
    save_result: String,
    compress: bool,
    pending_file: Pending<String>,
}

impl Default for GZip {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            save_result: String::new(),
            compress: false,
            pending_file: Pending::default(),
        }
    }
}

impl Tool for GZip {
    fn name(&self) -> String { tr!("gzip_name") }
    fn description(&self) -> String { tr!("gzip_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    self.input = String::from_utf8_lossy(&bytes).to_string();
                }
                Err(e) => self.error = tr!("err_file_read", e),
            }
        }
        if let Some(text) = self.pending_file.poll() {
            self.save_result = text;
        }

        let label_compress = tr!("label_compress");
        let label_decompress = tr!("label_decompress");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.compress, true, &label_compress);
            ui.radio_value(&mut self.compress, false, &label_decompress);
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_mode = self.compress;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match crate::clipboard::read_text() {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(&tr!("save_as_title"))
                            .add_filter(&tr!("save_filter_all"), &["*"])
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
                                Err(e) => self.error = tr!("err_file_read", e),
                            }
                        }
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                        self.save_result.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_input"));

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
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        save_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("default_output_txt"), self.output.clone());
                    }
                });
                if !self.save_result.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, &self.save_result);
                }
                ui.add_space(2.0);
                let out_label = if self.compress { tr!("gzip_compressed_label") } else { tr!("gzip_decompressed_label") };
                ui.label(&out_label);

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
                    self.error = tr!("gzip_compress_error", e);
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
                        self.error = tr!("gzip_decompress_error", e);
                    }
                },
                Err(e) => {
                    self.output.clear();
                    self.error = tr!("gzip_b64_error", e);
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
