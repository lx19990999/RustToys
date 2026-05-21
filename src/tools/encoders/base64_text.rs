use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::Pending;
use base64::Engine;

#[derive(Default)]
pub struct Base64Text {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Tool for Base64Text {
    fn name(&self) -> String { tr!("b64t_name") }
    fn description(&self) -> String { tr!("b64t_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        let label_encode = tr!("label_encode");
        let label_decode = tr!("label_decode");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encode_mode, true, &label_encode);
            ui.radio_value(&mut self.encode_mode, false, &label_decode);
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_mode = self.encode_mode;

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
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title(&tr!("save_as_title"))
                            .add_filter(&tr!("save_filter_all"), &["*"])
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
                                Err(e) => self.error = tr!("err_file_read", e),
                            }
                        }
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_input"));

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
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("default_output_txt"), self.output.clone());
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_output"));

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
                    self.error = tr!("b64t_decode_error", e);
                }
            }
        }
    }
}
