use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use image::{GenericImageView, ImageEncoder};

const FORMATS: &[(&str, &str)] = &[
    ("PNG", "png"),
    ("JPEG", "jpg"),
    ("BMP", "bmp"),
    ("GIF", "gif"),
    ("WebP", "webp"),
];

pub struct ImageConverter {
    // Source
    file_path: String,
    source_info: String,
    error: String,

    // Image data
    loaded: bool,
    img_width: u32,
    img_height: u32,
    original_pixels: Vec<u8>,

    // Texture for preview
    texture: Option<egui::TextureHandle>,

    // Output settings
    format_index: usize,
    resize_enabled: bool,
    target_width: u32,
    target_height: u32,
    keep_aspect: bool,
    jpeg_quality: u8,

    // Status
    status: String,
}

impl Default for ImageConverter {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            source_info: String::new(),
            error: String::new(),
            loaded: false,
            img_width: 0,
            img_height: 0,
            original_pixels: Vec::new(),
            texture: None,
            format_index: 0,
            resize_enabled: false,
            target_width: 0,
            target_height: 0,
            keep_aspect: true,
            jpeg_quality: 85,
            status: String::new(),
        }
    }
}

impl ImageConverter {
    fn load_image(&mut self, path: std::path::PathBuf) {
        self.error.clear();
        self.status.clear();
        self.source_info.clear();
        self.file_path = path.to_string_lossy().to_string();

        match image::open(&path) {
            Ok(img) => {
                let (w, h) = img.dimensions();
                let rgba = img.to_rgba8();
                self.original_pixels = rgba.into_raw();
                self.img_width = w;
                self.img_height = h;
                self.target_width = w;
                self.target_height = h;
                self.loaded = true;
                self.texture = None;

                let file_size = std::fs::metadata(&path)
                    .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                    .unwrap_or_else(|_| "Unknown".to_string());

                self.source_info = format!(
                    "File: {}\nSize: {}\nDimensions: {} x {} px\nColor: {:?}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    file_size, w, h, img.color(),
                );
            }
            Err(e) => {
                self.error = format!("Failed to open image: {}", e);
                self.loaded = false;
            }
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if self.texture.is_some() {
            return;
        }
        if !self.loaded {
            return;
        }
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [self.img_width as usize, self.img_height as usize],
            &self.original_pixels,
        );
        self.texture = Some(ctx.load_texture("img_conv_preview", color_image, egui::TextureOptions::default()));
    }

    fn get_output_pixels(&self) -> Vec<u8> {
        if !self.resize_enabled || (self.target_width == self.img_width && self.target_height == self.img_height) {
            return self.original_pixels.clone();
        }
        let img = image::RgbaImage::from_raw(self.img_width, self.img_height, self.original_pixels.clone())
            .unwrap();
        let resized = image::imageops::resize(
            &img,
            self.target_width,
            self.target_height,
            image::imageops::FilterType::Lanczos3,
        );
        resized.into_raw()
    }

    fn save_image(&mut self) {
        self.error.clear();
        self.status.clear();
        if !self.loaded {
            return;
        }
        let pixels = self.get_output_pixels();
        let w = if self.resize_enabled { self.target_width } else { self.img_width };
        let h = if self.resize_enabled { self.target_height } else { self.img_height };

        let (name, ext) = FORMATS[self.format_index];
        if let Some(path) = crate::tools::async_utils::save_file_dialog("Save image as", name, &[ext], &format!("output.{}", ext)) {
            let img = image::RgbaImage::from_raw(w, h, pixels).unwrap();

            let result = if ext == "jpg" {
                // JPEG doesn't support alpha, convert to RGB
                let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
                let mut buf = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, self.jpeg_quality);
                if encoder.write_image(rgb_img.as_raw(), w, h, image::ColorType::Rgb8.into()).is_ok() {
                    std::fs::write(&path, buf).map_err(|e| e.to_string())
                } else {
                    Err("JPEG encoding failed".to_string())
                }
            } else {
                img.save(&path).map_err(|e| e.to_string())
            };

            match result {
                Ok(_) => {
                    let size = std::fs::metadata(&path)
                        .map(|m| format!("{:.1} KB", m.len() as f64 / 1024.0))
                        .unwrap_or_default();
                    self.status = format!("Saved: {} ({})", path.file_name().unwrap_or_default().to_string_lossy(), size);
                }
                Err(e) => {
                    self.error = format!("Save failed: {}", e);
                }
            }
        }
    }

    fn clear_selection(&mut self) {
        self.error.clear();
        self.status.clear();
        self.loaded = false;
        self.original_pixels.clear();
        self.texture = None;
        self.source_info.clear();
        self.file_path.clear();
        self.img_width = 0;
        self.img_height = 0;
        self.target_width = 0;
        self.target_height = 0;
    }
}

impl Tool for ImageConverter {
    fn name(&self) -> &str { "Image Converter" }
    fn description(&self) -> &str { "Convert, resize, and compress images between formats" }
    fn category(&self) -> ToolCategory { ToolCategory::Graphic }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // Load section
        ui.horizontal(|ui| {
            if ui.button("Open Image...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open image")
                    .add_filter("Image", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.load_image(path);
                }
            }
            if !self.file_path.is_empty() {
                ui.label(&self.file_path);
            }
        });
        ui.add_space(4.0);

        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::RED, &self.error);
        }
        if !self.status.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(0, 160, 0), &self.status);
        }

        if !self.loaded {
            return;
        }

        // Source info
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Source").strong());
            ui.label(&self.source_info);
        });

        // Preview
        ui.add_space(8.0);
        self.update_texture(ui.ctx());
        if let Some(ref tex) = self.texture {
            let max_w = ui.available_width().min(500.0);
            let aspect = self.img_height as f32 / self.img_width as f32;
            let dw = max_w;
            let dh = max_w * aspect;
            let max_h = 300.0;
            let (dw, dh) = if dh > max_h { (max_h / aspect, max_h) } else { (dw, dh) };
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::new(egui::load::SizedTexture::new(
                    tex.id(),
                    egui::Vec2::new(dw, dh),
                )));
            });
        }

        // Output settings
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Output Settings").strong());
            ui.add_space(4.0);

            // Format
            ui.horizontal(|ui| {
                ui.label("Format:");
                egui::ComboBox::from_id_salt("img_format")
                    .selected_text(FORMATS[self.format_index].0)
                    .show_ui(ui, |ui| {
                        for (i, (name, _)) in FORMATS.iter().enumerate() {
                            ui.selectable_value(&mut self.format_index, i, *name);
                        }
                    });
            });
            ui.add_space(4.0);

            // Resize
            ui.checkbox(&mut self.resize_enabled, "Resize");
            if self.resize_enabled {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    if ui.add(egui::DragValue::new(&mut self.target_width).range(1..=16384).speed(1)).changed() {
                        if self.keep_aspect && self.img_width > 0 {
                            self.target_height = (self.target_width as f64 * self.img_height as f64 / self.img_width as f64) as u32;
                        }
                    }
                    ui.label("px");
                    ui.separator();
                    ui.label("Height:");
                    if ui.add(egui::DragValue::new(&mut self.target_height).range(1..=16384).speed(1)).changed() {
                        if self.keep_aspect && self.img_height > 0 {
                            self.target_width = (self.target_height as f64 * self.img_width as f64 / self.img_height as f64) as u32;
                        }
                    }
                    ui.label("px");
                });
                ui.checkbox(&mut self.keep_aspect, "Keep aspect ratio");
            }

            // JPEG quality
            if FORMATS[self.format_index].1 == "jpg" {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("JPEG quality:");
                    ui.add(egui::Slider::new(&mut self.jpeg_quality, 1..=100).suffix("%"));
                });
            }
        });

        // Action buttons
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Save As...").clicked() {
                self.save_image();
            }
            ui.separator();
            if ui.button("Clear").clicked() {
                self.clear_selection();
            }
        });
    }
}
