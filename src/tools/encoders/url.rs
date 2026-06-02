use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};
use crate::tools::io_layout;


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

        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            crate::tools::async_utils::open_dropped_text_async(&mut self.pending_file, path);
        }
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

        let lbl_paste = tr!("btn_paste");
        let lbl_open_file = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_multiline = tr!("label_multiline");
        let lbl_input = tr!("label_input");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_output = tr!("label_output");
        let lbl_save_title = tr!("save_as_title");
        let lbl_save_filter = tr!("save_filter_text");
        let lbl_default_output = tr!("default_output_txt");
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
                        open_file_async(&mut self.pending_file, &lbl_save_title, &lbl_save_filter, &["txt"]);
                    }
                    if ui.button(&lbl_clear).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(io_layout::ROW_GAP);
                ui.checkbox(&mut self.multiline, &lbl_multiline);
                ui.add_space(io_layout::ROW_GAP);
                ui.label(&lbl_input);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "url_input_scroll", &mut self.input);
            }
            io_layout::IoColumn::Right => {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                        crate::tools::async_utils::save_file_async(
                            &mut self.save_pending,
                            &lbl_save_title,
                            &lbl_save_filter,
                            &["txt"],
                            &lbl_default_output,
                            self.output.clone(),
                        );
                    }
                });
                io_layout::row_spacer(ui, opt_h + io_layout::ROW_GAP);
                ui.label(&lbl_output);
                ui.add_space(io_layout::ROW_GAP);
                io_layout::multiline_field(ui, w, "url_output_scroll", &mut self.output);
            }
        });

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
