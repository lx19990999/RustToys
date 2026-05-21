use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};


pub struct UrlEncoder {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
    multiline: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for UrlEncoder {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encode_mode: false,
            multiline: false,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}


impl Tool for UrlEncoder {
    fn name(&self) -> String { tr!("url_name") }
    fn description(&self) -> String { tr!("url_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_mode = self.encode_mode;
        let prev_multiline = self.multiline;

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
                        open_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.checkbox(&mut self.multiline, tr!("label_multiline"));
                ui.add_space(2.0);
                ui.label(tr!("label_input"));

                egui::ScrollArea::vertical()
                    .id_salt("url_input_scroll")
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
                    .id_salt("url_output_scroll")
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
        if self.input != prev_input || self.encode_mode != prev_mode || self.multiline != prev_multiline {
            self.convert();
        }
    }
}

impl UrlEncoder {
    fn convert(&mut self) {
        self.error.clear();
        if self.input.is_empty() {
            self.output.clear();
            return;
        }

        if self.multiline {
            let lines: Vec<String> = self.input.lines().map(|line| {
                if self.encode_mode {
                    urlencoding::encode(line).to_string()
                } else {
                    String::from_utf8_lossy(&urlencoding::decode_binary(line.as_bytes())).to_string()
                }
            }).collect();
            self.output = lines.join("\n");
        } else if self.encode_mode {
            self.output = urlencoding::encode(&self.input).to_string();
        } else {
            self.output = String::from_utf8_lossy(&urlencoding::decode_binary(self.input.as_bytes())).to_string();
        }
    }
}
