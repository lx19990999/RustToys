use eframe::egui;
use crate::tool::{Tool, ToolCategory};
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
        }
    }
}


impl Tool for QrCode {
    fn name(&self) -> &str { "QR Code Encoder / Decoder" }
    fn description(&self) -> &str { "Generate QR codes from text or decode from image files" }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.input = text;
            }
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.mode, false, "Encode");
            ui.radio_value(&mut self.mode, true, "Decode");
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
                        self.svg_output.clear();
                        self.png_bytes.clear();
                        self.error.clear();
                        self.preview_texture = None;
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    ui.label("Error correction:");
                    ui.radio_value(&mut self.ecc_level, 0, "L (7%)");
                    ui.radio_value(&mut self.ecc_level, 1, "M (15%)");
                    ui.radio_value(&mut self.ecc_level, 2, "Q (25%)");
                    ui.radio_value(&mut self.ecc_level, 3, "H (30%)");
                });
                ui.add_space(2.0);
                ui.label("Input:");

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
            if self.input != prev_input || self.ecc_level != prev_ecc {
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
                        if ui.button("Copy PNG").clicked() {
                            let b64 = base64::Engine::encode(
                                &base64::engine::general_purpose::STANDARD,
                                &self.png_bytes,
                            );
                            ui.ctx().copy_text(b64);
                        }
                        if ui.button("Save As PNG...").clicked() {
                            if let Some(path) = crate::tools::async_utils::save_file_dialog("Save QR code as PNG", "PNG", &["png"], "qrcode.png") {
                                let _ = std::fs::write(path, &self.png_bytes);
                            }
                        }
                        if ui.button("Save As SVG...").clicked() && !self.svg_output.is_empty() {
                            if let Some(path) = crate::tools::async_utils::save_file_dialog("Save QR code as SVG", "SVG", &["svg"], "qrcode.svg") {
                                let _ = std::fs::write(path, &self.svg_output);
                            }
                        }
                    });
                } else if !self.input.trim().is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label("Generating...");
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label("Enter text to generate a QR code.");
                    });
                }
            });
        });
    }

    fn ui_decode(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Open Image...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open QR code image")
                    .add_filter("Image", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.decode_file_path = path.to_string_lossy().to_string();
                    self.decode_from_file(ui.ctx());
                }
            }
            if ui.button("Paste Image").clicked() {
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
                    Err(e) => self.decode_error = format!("Clipboard image error: {}", e),
                }
            }
            if ui.button("Clear").clicked() {
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
                    ui.label(format!("File: {}", self.decode_file_path));
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
                    ui.label("Open or paste a QR code image to decode.");
                }
            });

            // Right: Decoded text
            cols[1].vertical(|ui| {
                if !self.decode_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.decode_error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() && !self.decoded_text.is_empty() {
                        ui.ctx().copy_text(self.decoded_text.clone());
                    }
                    if ui.button("Save As...").clicked() && !self.decoded_text.is_empty() {
                        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save decoded text as", "Text", &["txt"], "decoded.txt") {
                            let _ = std::fs::write(path, &self.decoded_text);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label("Decoded Text:");

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
                self.error = format!("QR generation error: {}", e);
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
                    Err(e) => self.decode_error = format!("Cannot open image: {}", e),
                }
            }
            Err(e) => self.decode_error = format!("File read error: {}", e),
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
            self.decode_error = "No QR code detected in image".to_string();
            return;
        }

        let mut results = Vec::new();
        for grid in grids {
            match grid.decode() {
                Ok((_meta, content)) => results.push(content),
                Err(e) => results.push(format!("[decode error: {}]", e)),
            }
        }

        self.decoded_text = results.join("\n---\n");
    }
}
