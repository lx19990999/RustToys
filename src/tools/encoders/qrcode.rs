use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};


pub struct QrCode {
    // Encode
    input: String,
    svg_output: String,
    png_bytes: Vec<u8>,
    ecc_level: usize, // 0=L, 1=M, 2=Q, 3=H
    error: String,
    mode: bool, // false=encode, true=decode
    preview_texture: Option<egui::TextureHandle>,
    // Decode
    decode_file_path: String,
    decoded_text: String,
    decode_error: String,
    decode_preview: Option<egui::TextureHandle>,
    pending_file: Pending<String>,
    file_loaded: bool,
}

impl Default for QrCode {
    fn default() -> Self {
        Self {
            input: String::new(),
            svg_output: String::new(),
            png_bytes: Vec::new(),
            ecc_level: 0,
            error: String::new(),
            mode: false,
            preview_texture: None,
            decode_file_path: String::new(),
            decoded_text: String::new(),
            decode_error: String::new(),
            decode_preview: None,
            pending_file: Pending::default(),
            file_loaded: false,
        }
    }
}


impl Tool for QrCode {
    fn name(&self) -> String { tr!("qr_name") }
    fn description(&self) -> String { tr!("qr_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.file_loaded = false;
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
                self.file_loaded = true;
            }
        }
        let label_encode = tr!("label_encode");
        let label_decode = tr!("label_decode");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.mode, false, &label_encode);
            ui.radio_value(&mut self.mode, true, &label_decode);
        });
        ui.add_space(4.0);

        if self.mode {
            self.ui_decode(ui);
        } else {
            self.ui_encode(ui);
        }
    }
}

impl QrCode {
    fn ui_encode(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_ecc = self.ecc_level;

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
                        self.svg_output.clear();
                        self.png_bytes.clear();
                        self.error.clear();
                        self.preview_texture = None;
                    }
                });
                ui.add_space(2.0);

                let ecc_l = tr!("qr_ecc_l");
                let ecc_m = tr!("qr_ecc_m");
                let ecc_q = tr!("qr_ecc_q");
                let ecc_h = tr!("qr_ecc_h");
                ui.horizontal(|ui| {
                    ui.label(tr!("qr_ecc_label"));
                    ui.radio_value(&mut self.ecc_level, 0, &ecc_l);
                    ui.radio_value(&mut self.ecc_level, 1, &ecc_m);
                    ui.radio_value(&mut self.ecc_level, 2, &ecc_q);
                    ui.radio_value(&mut self.ecc_level, 3, &ecc_h);
                });
                ui.add_space(2.0);
                ui.label(tr!("label_input"));

                egui::ScrollArea::vertical()
                    .id_salt("qr_encode_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Auto-convert before rendering preview
            if self.file_loaded || self.input != prev_input || self.ecc_level != prev_ecc {
                self.generate_qr(cols[1].ctx());
            }

            // Right: Preview
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                if let Some(tex) = &self.preview_texture {
                    let max_w = ui.available_width().min(400.0);
                    let scale = max_w / tex.size_vec2().x;
                    let size = tex.size_vec2() * scale.min(1.0);
                    ui.vertical_centered(|ui| {
                        ui.image((tex.id(), size));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button(tr!("btn_copy_png")).clicked() {
                            let b64 = base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &self.png_bytes,
                            );
                            ui.ctx().copy_text(b64);
                        }
                        if ui.button(tr!("btn_save_png")).clicked() {
                            if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("qr_save_png_title"), "PNG", &["png"], &tr!("qr_save_png")) {
                                let _ = std::fs::write(path, &self.png_bytes);
                            }
                        }
                        if ui.button(tr!("btn_save_svg")).clicked() && !self.svg_output.is_empty() {
                            if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("qr_save_svg_title"), "SVG", &["svg"], &tr!("qr_save_svg")) {
                                let _ = std::fs::write(path, &self.svg_output);
                            }
                        }
                    });
                } else if !self.input.trim().is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(tr!("qr_generating"));
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(tr!("qr_enter_text"));
                    });
                }
            });
        });
    }

    fn ui_decode(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(tr!("btn_open_image")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(&tr!("qr_open_title"))
                    .add_filter(&tr!("cb_filter_image"), &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .add_filter(&tr!("save_filter_all"), &["*"])
                    .pick_file()
                {
                    self.decode_file_path = path.to_string_lossy().to_string();
                    self.decode_from_file(ui.ctx());
                }
            }
            if ui.button(tr!("btn_paste_image")).clicked() {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_image()) {
                    Ok(img) => {
                        let rgba = img.bytes;
                        let w = img.width;
                        let h = img.height;
                        // Load preview
                        let color_img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                        self.decode_preview = Some(ui.ctx().load_texture(
                            "qr_decode_preview", color_img, Default::default(),
                        ));
                        // Decode QR from image
                        self.decode_qr_from_rgba(&rgba, w as u32, h as u32);
                        self.decode_error.clear();
                    }
                    Err(e) => self.decode_error = tr!("err_clipboard_image", e),
                }
            }
            if ui.button(tr!("btn_clear")).clicked() {
                self.decoded_text.clear();
                self.decode_error.clear();
                self.decode_file_path.clear();
                self.decode_preview = None;
            }
        });
        ui.add_space(4.0);

        ui.columns(2, |cols| {
            // Left: Image preview
            cols[0].vertical(|ui| {
                if !self.decode_file_path.is_empty() {
                    ui.label(tr!("b64i_file_label", self.decode_file_path));
                }
                if let Some(tex) = &self.decode_preview {
                    ui.add_space(4.0);
                    let max_w = ui.available_width().min(300.0);
                    let scale = max_w / tex.size_vec2().x;
                    let size = tex.size_vec2() * scale.min(1.0);
                    ui.vertical_centered(|ui| {
                        ui.image((tex.id(), size));
                    });
                } else {
                    ui.label(tr!("qr_paste_image_hint"));
                }
            });

            // Right: Decoded text
            cols[1].vertical(|ui| {
                if !self.decode_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.decode_error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.decoded_text.is_empty() {
                        ui.ctx().copy_text(self.decoded_text.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.decoded_text.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("qr_save_decoded"), &tr!("save_filter_text"), &["txt"], &tr!("qr_decoded_txt")) {
                            let _ = std::fs::write(path, &self.decoded_text);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("qr_decoded_label"));

                egui::ScrollArea::vertical()
                    .id_salt("qr_decode_output_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.decoded_text)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });
    }

    fn generate_qr(&mut self, ctx: &egui::Context) {
        self.error.clear();
        self.svg_output.clear();
        self.png_bytes.clear();
        self.preview_texture = None;

        if self.input.trim().is_empty() {
            return;
        }

        let ecc = match self.ecc_level {
            0 => qrcode_generator::QrCodeEcc::Low,
            1 => qrcode_generator::QrCodeEcc::Medium,
            2 => qrcode_generator::QrCodeEcc::Quartile,
            _ => qrcode_generator::QrCodeEcc::High,
        };

        // Get QR code matrix
        let matrix = match qrcode_generator::to_matrix(&self.input, ecc) {
            Ok(v) => v,
            Err(e) => {
                self.error = tr!("qr_gen_error", e);
                return;
            }
        };
        let size = matrix.len();

        // Generate SVG
        self.svg_output = qrcode_generator::to_svg_to_string(&self.input, ecc, 256, None::<&str>)
            .unwrap_or_default();

        // Render PNG from matrix
        let module_px: u32 = 10;
        let margin: u32 = 4;
        let img_size = size as u32 * module_px + margin * 2;
        let mut img_buf = image::RgbImage::from_pixel(img_size, img_size, image::Rgb([255u8, 255, 255]));
        for (y, row) in matrix.iter().enumerate() {
            for (x, &dark) in row.iter().enumerate() {
                if dark {
                    for dy in 0..module_px {
                        for dx in 0..module_px {
                            img_buf.put_pixel(
                                margin + x as u32 * module_px + dx,
                                margin + y as u32 * module_px + dy,
                                image::Rgb([0, 0, 0]),
                            );
                        }
                    }
                }
            }
        }

        // Encode as PNG
        let mut png_cursor = std::io::Cursor::new(Vec::new());
        let dynamic_img = image::DynamicImage::ImageRgb8(img_buf);
        if dynamic_img.write_to(&mut png_cursor, image::ImageFormat::Png).is_ok() {
            let png_data = png_cursor.into_inner();

            // Create egui texture
            let rgba = dynamic_img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize], &rgba,
            );
            self.preview_texture = Some(ctx.load_texture(
                "qr_preview", color_img, Default::default(),
            ));
            self.png_bytes = png_data;
        }
    }

    fn decode_from_file(&mut self, ctx: &egui::Context) {
        self.decode_error.clear();
        self.decoded_text.clear();
        self.decode_preview = None;

        if self.decode_file_path.is_empty() {
            return;
        }

        match std::fs::read(&self.decode_file_path) {
            Ok(bytes) => {
                match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        // Load preview
                        let color_img = egui::ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize], &rgba,
                        );
                        self.decode_preview = Some(ctx.load_texture(
                            "qr_decode_preview", color_img, Default::default(),
                        ));
                        // Decode
                        self.decode_qr_from_rgba(&rgba, w, h);
                    }
                    Err(e) => self.decode_error = tr!("qr_cannot_open", e),
                }
            }
            Err(e) => self.decode_error = tr!("err_file_read", e),
        }
    }

    fn decode_qr_from_rgba(&mut self, rgba: &[u8], width: u32, height: u32) {
        self.decoded_text.clear();

        // Convert RGBA to luma8 for rqrr
        let mut gray = Vec::with_capacity((width * height) as usize);
        for chunk in rgba.chunks(4) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            let luma = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            gray.push(luma);
        }

        let mut img = rqrr::PreparedImage::prepare_from_greyscale(
            width as usize,
            height as usize,
            |x, y| gray[y * width as usize + x],
        );

        let grids = img.detect_grids();
        if grids.is_empty() {
            self.decode_error = tr!("qr_no_detected");
            return;
        }

        let mut results = Vec::new();
        for grid in grids {
            match grid.decode() {
                Ok((_meta, content)) => results.push(content),
                Err(e) => results.push(tr!("qr_decode_error_fmt", e)),
            }
        }

        self.decoded_text = results.join("\n---\n");
    }
}
