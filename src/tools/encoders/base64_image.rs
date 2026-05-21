use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};
use base64::Engine;


pub struct Base64Image {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
    preview_texture: Option<egui::TextureHandle>,
    image_info: String,
    pending_file: Pending<String>,
}

impl Default for Base64Image {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encode_mode: false,
            preview_texture: None,
            image_info: String::new(),
            pending_file: Pending::default(),
        }
    }
}


impl Tool for Base64Image {
    fn name(&self) -> String { tr!("b64i_name") }
    fn description(&self) -> String { tr!("b64i_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
            }
        }
        let label_encode_mode = tr!("b64i_encode_mode");
        let label_decode_mode = tr!("b64i_decode_mode");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encode_mode, true, &label_encode_mode);
            ui.radio_value(&mut self.encode_mode, false, &label_decode_mode);
        });
        ui.add_space(4.0);

        let prev_input = self.input.clone();
        let prev_mode = self.encode_mode;

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                if self.encode_mode {
                    ui.horizontal(|ui| {
                        if ui.button(tr!("btn_open_image")).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title(&tr!("cb_open_title"))
                                .add_filter(&tr!("cb_filter_image"), &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                                .add_filter(&tr!("save_filter_all"), &["*"])
                                .pick_file()
                            {
                                match std::fs::read(&path) {
                                    Ok(bytes) => {
                                        self.input = path.to_string_lossy().to_string();
                                        self.load_preview(&bytes, ui.ctx());
                                        self.output = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                        self.error.clear();
                                    }
                                    Err(e) => self.error = tr!("err_file_read", e),
                                }
                            }
                        }
                        if ui.button(tr!("btn_clear")).clicked() {
                            self.reset();
                        }
                    });
                    ui.add_space(2.0);

                    // Show file path or preview
                    if !self.input.is_empty() {
                        ui.label(tr!("b64i_file_label", self.input));
                        ui.label(&self.image_info);
                    }

                    if let Some(tex) = &self.preview_texture {
                        ui.add_space(4.0);
                        let max_w = ui.available_width().min(300.0);
                        let scale = max_w / tex.size_vec2().x;
                        let size = tex.size_vec2() * scale.min(1.0);
                        ui.image((tex.id(), size));
                    }
                } else {
                    // Decode mode
                    ui.horizontal(|ui| {
                        if ui.button(tr!("btn_paste")).clicked() {
                            match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                                Ok(text) => {
                                    self.input = text;
                                    self.preview_texture = None;
                                    self.image_info.clear();
                                }
                                Err(e) => self.error = tr!("err_clipboard", e),
                            }
                        }
                        if ui.button(tr!("btn_paste_image")).clicked() {
                            match arboard::Clipboard::new().and_then(|mut cb| cb.get_image()) {
                                Ok(img) => {
                                    let rgba = img.bytes;
                                    let w = img.width;
                                    let h = img.height;
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                        [w, h], &rgba,
                                    );
                                    self.preview_texture = Some(ui.ctx().load_texture(
                                        "b64img_preview", color_img, Default::default(),
                                    ));
                                    self.image_info = format!("{}x{} ({} bytes)", w, h, rgba.len());
                                    self.output = base64::engine::general_purpose::STANDARD.encode(&rgba);
                                    self.error.clear();
                                }
                                Err(e) => self.error = tr!("err_clipboard_image", e),
                            }
                        }
                        if ui.button(tr!("btn_open_file")).clicked() {
                            open_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"]);
                        }
                        if ui.button(tr!("btn_clear")).clicked() {
                            self.reset();
                        }
                    });
                    ui.add_space(2.0);
                    ui.label(tr!("b64i_base64_input"));

                    egui::ScrollArea::vertical()
                        .id_salt("b64img_input_scroll")
                        .auto_shrink([false, false])
                        .max_height(150.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.input)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                }
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
                        let default_name = if self.encode_mode { tr!("default_output_txt") } else { "output.png".to_string() };
                        let filter_name = if self.encode_mode { tr!("save_filter_text") } else { "PNG".to_string() };
                        let exts: &[&str] = if self.encode_mode { &["txt"] } else { &["png"] };
                        if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("save_as_title"), &filter_name, exts, &default_name) {
                            if self.encode_mode {
                                let _ = std::fs::write(path, &self.output);
                            } else {
                                let cleaned = self.clean_base64(&self.input);
                                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(cleaned.as_ref()) {
                                    let _ = std::fs::write(path, &bytes);
                                }
                            }
                        }
                    }
                });
                ui.add_space(2.0);

                if self.encode_mode {
                    ui.label(tr!("b64i_base64_output"));
                    egui::ScrollArea::vertical()
                        .id_salt("b64img_output_scroll")
                        .auto_shrink([false, false])
                        .max_height(150.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.output)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                } else {
                    ui.label(&self.image_info);

                    // Image preview
                    if let Some(tex) = &self.preview_texture {
                        let max_w = ui.available_width().min(500.0);
                        let scale = max_w / tex.size_vec2().x;
                        let size = tex.size_vec2() * scale.min(1.0);
                        ui.add_space(4.0);
                        egui::ScrollArea::both()
                            .id_salt("b64img_preview_scroll")
                            .auto_shrink([false, false])
                            .max_height(ui.available_height())
                            .show(ui, |ui| {
                                ui.image((tex.id(), size));
                            });
                    } else if !self.output.is_empty() {
                        ui.label(&self.output);
                    }
                }
            });
        });

        // Auto-convert on change
        if self.input != prev_input || self.encode_mode != prev_mode {
            if !self.encode_mode {
                self.decode_to_preview(ui);
            }
        }
    }
}

impl Base64Image {
    fn reset(&mut self) {
        self.input.clear();
        self.output.clear();
        self.error.clear();
        self.image_info.clear();
        self.preview_texture = None;
    }

    fn clean_base64<'a>(&self, raw: &'a str) -> std::borrow::Cow<'a, str> {
        let s = raw.trim();
        if let Some(stripped) = s.strip_prefix("data:") {
            if let Some(idx) = stripped.find("base64,") {
                return std::borrow::Cow::Owned(stripped[idx + 7..].to_string());
            }
        }
        std::borrow::Cow::Owned(s.chars().filter(|c| !c.is_whitespace()).collect())
    }

    fn load_preview(&mut self, bytes: &[u8], ctx: &egui::Context) {
        match image::load_from_memory(bytes) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                self.image_info = tr!("b64i_image_info", w, h, bytes.len() as f64 / 1024.0);
                let color_img = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize], &rgba,
                );
                self.preview_texture = Some(ctx.load_texture(
                    "b64img_preview", color_img, Default::default(),
                ));
            }
            Err(e) => {
                self.image_info = tr!("b64i_cannot_preview", bytes.len() as f64 / 1024.0, e);
                self.preview_texture = None;
            }
        }
    }

    fn decode_to_preview(&mut self, ui: &mut egui::Ui) {
        self.error.clear();
        self.output.clear();
        if self.input.trim().is_empty() {
            self.preview_texture = None;
            self.image_info.clear();
            return;
        }

        let cleaned = self.clean_base64(&self.input);
        match base64::engine::general_purpose::STANDARD.decode(cleaned.as_bytes()) {
            Ok(bytes) => {
                self.output = tr!("b64i_decoded_bytes", bytes.len());
                self.load_preview(&bytes, ui.ctx());
            }
            Err(e) => {
                self.error = tr!("b64i_decode_error", e);
                self.preview_texture = None;
                self.image_info.clear();
            }
        }
    }
}
