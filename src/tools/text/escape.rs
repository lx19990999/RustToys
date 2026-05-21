use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct EscapeUnescape {
    input: String,
    output: String,
    escape_mode: bool,
    prev_input: String,
    prev_mode: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
    save_result: String,
}

impl Default for EscapeUnescape {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            escape_mode: true,
            prev_input: String::new(),
            prev_mode: true,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
            save_result: String::new(),
        }
    }
}

impl Tool for EscapeUnescape {
    fn name(&self) -> String { tr!("esc_name") }
    fn description(&self) -> String { tr!("esc_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Text }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let err_reading = tr!("err_error_reading");
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&err_reading) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.save_result = text;
        }
        let total = ui.available_rect_before_wrap();
        let pad = 4.0;
        let w = total.width();
        let half_w = (w - pad) * 0.5;
        let h = total.height();

        let label_h = 18.0;
        let btn_h = 22.0;
        let space = 2.0;
        let mode_h = 22.0 + space;
        let top_h = label_h + space + btn_h * 2.0 + space * 3.0;

        let cols_h = (h - mode_h - pad * 2.0).max(120.0);

        // Auto-convert
        self.auto_convert();

        // --- Mode selector ---
        let mode_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(w, mode_h),
        );
        let lbl_escape = tr!("esc_escape");
        let lbl_unescape = tr!("esc_unescape");
        ui.scope_builder(egui::UiBuilder::new().max_rect(mode_rect), |ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.escape_mode, true, &lbl_escape);
                ui.radio_value(&mut self.escape_mode, false, &lbl_unescape);
            });
        });

        let cols_y = total.min.y + mode_h + pad;

        // --- Left: Input ---
        let left_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, cols_y),
            egui::vec2(half_w, cols_h),
        );
        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_input = tr!("label_input");
        let lbl_output = tr!("label_output");
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_input).strong());
            ui.add_space(space);
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_paste).clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.input = text,
                        Err(e) => self.output = tr!("err_clipboard", e),
                    }
                }
                if ui.button(&lbl_open).clicked() {
                    open_file_async(&mut self.pending_file, &tr!("btn_open_file"), &tr!("save_filter_text"), &["txt"]);
                }
                    if ui.button(&lbl_clear).clicked() {
                    self.input.clear();
                    self.output.clear();
                }
                if ui.button(&lbl_copy).clicked() && !self.input.is_empty() {
                    ui.ctx().copy_text(self.input.clone());
                }
                if ui.button(&lbl_save_as).clicked() && !self.input.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("esc_save_input"), self.input.clone());
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.input)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Right: Output ---
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x + half_w + pad, cols_y),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_output).strong());
            ui.add_space(space);
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_copy).clicked() && !self.output.is_empty() {
                    ui.ctx().copy_text(self.output.clone());
                }
                if ui.button(&lbl_save_as).clicked() && !self.output.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("esc_save_output"), self.output.clone());
                }
                if !self.save_result.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.save_result);
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.output)
                    .font(egui::TextStyle::Monospace),
            );
        });
    }
}

impl EscapeUnescape {
    fn auto_convert(&mut self) {
        if self.input != self.prev_input || self.escape_mode != self.prev_mode {
            self.prev_input = self.input.clone();
            self.prev_mode = self.escape_mode;
            if self.input.is_empty() {
                self.output.clear();
            } else if self.escape_mode {
                self.output = self.escape_string(&self.input);
            } else {
                self.output = self.unescape_string(&self.input);
            }
        }
    }

    fn escape_string(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 2);
        for c in input.chars() {
            match c {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                '\\' => result.push_str("\\\\"),
                '"' => result.push_str("\\\""),
                '\'' => result.push_str("\\'"),
                '\0' => result.push_str("\\0"),
                _ if c.is_control() => result.push_str(&format!("\\u{{{:04x}}}", c as u32)),
                _ => result.push(c),
            }
        }
        result
    }

    fn unescape_string(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some('0') => result.push('\0'),
                    Some('u') => {
                        if chars.peek() == Some(&'{') {
                            chars.next();
                            let mut hex = String::new();
                            while let Some(&hc) = chars.peek() {
                                if hc == '}' {
                                    chars.next();
                                    break;
                                }
                                hex.push(hc);
                                chars.next();
                            }
                            if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                                if let Some(ch) = char::from_u32(cp) {
                                    result.push(ch);
                                }
                            }
                        }
                    }
                    Some('x') => {
                        let mut hex = String::new();
                        for _ in 0..2 {
                            if let Some(&hc) = chars.peek() {
                                if hc.is_ascii_hexdigit() {
                                    hex.push(hc);
                                    chars.next();
                                }
                            }
                        }
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        }
                    }
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}
