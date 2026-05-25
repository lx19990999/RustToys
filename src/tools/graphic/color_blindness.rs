use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, save_file_binary_async};
use image::{GenericImageView, ImageEncoder};

const PROTANOPIA_MATRIX: [[f64; 3]; 3] = [
    [0.567, 0.433, 0.0],
    [0.558, 0.442, 0.0],
    [0.0, 0.242, 0.758],
];
const DEUTERANOPIA_MATRIX: [[f64; 3]; 3] = [
    [0.625, 0.375, 0.0],
    [0.7, 0.3, 0.0],
    [0.0, 0.3, 0.7],
];
const TRITANOPIA_MATRIX: [[f64; 3]; 3] = [
    [0.95, 0.05, 0.0],
    [0.0, 0.433, 0.567],
    [0.0, 0.475, 0.525],
];

fn simulate_image(pixels: &[u8], width: u32, height: u32, matrix: [[f64; 3]; 3]) -> Vec<u8> {
    let mut result = Vec::with_capacity(pixels.len());
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize * 4;
            let r = pixels[idx];
            let g = pixels[idx + 1];
            let b = pixels[idx + 2];
            let a = pixels[idx + 3];
            let rf = r as f64 / 255.0;
            let gf = g as f64 / 255.0;
            let bf = b as f64 / 255.0;
            let nr = (matrix[0][0] * rf + matrix[0][1] * gf + matrix[0][2] * bf).clamp(0.0, 1.0);
            let ng = (matrix[1][0] * rf + matrix[1][1] * gf + matrix[1][2] * bf).clamp(0.0, 1.0);
            let nb = (matrix[2][0] * rf + matrix[2][1] * gf + matrix[2][2] * bf).clamp(0.0, 1.0);
            result.extend_from_slice(&[(nr * 255.0) as u8, (ng * 255.0) as u8, (nb * 255.0) as u8, a]);
        }
    }
    result
}

pub struct ColorBlindness {
    error: String,
    image_loaded: bool,
    img_width: u32,
    img_height: u32,
    pixel_sets: [Vec<u8>; 4], // original, proto, deuto, trito
    textures: [Option<egui::TextureHandle>; 4],
    textures_dirty: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for ColorBlindness {
    fn default() -> Self {
        Self {
            error: String::new(),
            image_loaded: false,
            img_width: 0,
            img_height: 0,
            pixel_sets: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            textures: [None, None, None, None],
            textures_dirty: false,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl ColorBlindness {
    fn load_image(&mut self) {
        let title = tr!("cb_open_title");
        let filter_image = tr!("cb_filter_image");
        let filter_all = tr!("save_filter_all");
        if let Some(path) = rfd::FileDialog::new()
            .set_title(&title)
            .add_filter(&filter_image, &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .add_filter(&filter_all, &["*"])
            .pick_file()
        {
            self.error.clear();
            match image::open(&path) {
                Ok(img) => {
                    let (w, h) = img.dimensions();
                    let rgba = img.to_rgba8();
                    let raw = rgba.into_raw();

                    self.pixel_sets[0] = raw.clone();
                    self.pixel_sets[1] = simulate_image(&raw, w, h, PROTANOPIA_MATRIX);
                    self.pixel_sets[2] = simulate_image(&raw, w, h, DEUTERANOPIA_MATRIX);
                    self.pixel_sets[3] = simulate_image(&raw, w, h, TRITANOPIA_MATRIX);

                    self.img_width = w;
                    self.img_height = h;
                    self.image_loaded = true;
                    self.textures_dirty = true;
                    self.textures = [None, None, None, None];
                }
                Err(e) => self.error = tr!("cb_failed_open", e),
            }
        }
    }

    fn ensure_textures(&mut self, ctx: &egui::Context) {
        if !self.textures_dirty {
            return;
        }
        self.textures_dirty = false;
        for i in 0..4 {
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [self.img_width as usize, self.img_height as usize],
                &self.pixel_sets[i],
            );
            if let Some(ref mut tex) = self.textures[i] {
                tex.set(color_image, egui::TextureOptions::default());
            } else {
                let name = format!("cb_sim_{}", i);
                self.textures[i] = Some(ctx.load_texture(&name, color_image, egui::TextureOptions::default()));
            }
        }
    }

    /// Fixed height for column titles (two lines) so preview images align across columns.
    fn column_title_height(ui: &egui::Ui) -> f32 {
        let font_id = egui::FontId::proportional(13.0);
        let line_h = ui.fonts(|f| f.row_height(&font_id));
        line_h * 2.0 + ui.spacing().item_spacing.y
    }

    fn show_column_title(ui: &mut egui::Ui, col_w: f32, title_h: f32, label: &str) {
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, title_h),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.set_width(col_w);
                ui.set_min_height(title_h);
                ui.set_max_height(title_h);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(label).strong().size(13.0));
                });
            },
        );
    }

    /// Scale image to fit within the column's available width and height.
    fn show_image(ui: &mut egui::Ui, tex: &egui::TextureHandle, w: u32, h: u32) {
        let max_w = ui.available_width().max(1.0);
        let max_h = ui.available_height().max(1.0);
        let aspect = h as f32 / w.max(1) as f32;

        let mut dw = max_w;
        let mut dh = max_w * aspect;
        if dh > max_h {
            dh = max_h;
            dw = max_h / aspect;
        }
        if dw > max_w {
            dw = max_w;
            dh = max_w * aspect;
        }

        ui.add(egui::Image::new(egui::load::SizedTexture::new(
            tex.id(),
            egui::Vec2::new(dw, dh),
        )));
    }

}

impl Tool for ColorBlindness {
    fn name(&self) -> String { tr!("cb_name") }
    fn description(&self) -> String { tr!("cb_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Graphic }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            self.error = text;
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }

        ui.horizontal(|ui| {
            let lbl_open = tr!("btn_open_file");
            if ui.button(lbl_open).clicked() {
                self.load_image();
            }
            if self.image_loaded {
                ui.label(tr!("cb_dimensions", self.img_width, self.img_height));
            }
        });

        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::RED, &self.error);
        }

        if self.image_loaded {
            self.ensure_textures(ui.ctx());

            let lbl_original = tr!("cb_original");
            let lbl_protanopia = tr!("cb_protanopia");
            let lbl_deuteranopia = tr!("cb_deuteranopia");
            let lbl_tritanopia = tr!("cb_tritanopia");
            let labels = [&lbl_original, &lbl_protanopia, &lbl_deuteranopia, &lbl_tritanopia];

            ui.add_space(8.0);
            let lbl_save_as = tr!("btn_save_as");
            let title = tr!("save_as_title");
            let filter_image = tr!("cb_filter_image");
            let default_name = tr!("cb_save_default");

            let panel_h = ui.available_height().max(200.0);
            let title_h = Self::column_title_height(ui);
            let btn_h = ui.spacing().interact_size.y;
            let img_h = (panel_h - title_h - btn_h - 12.0).max(80.0);

            ui.columns(4, |columns| {
                for i in 0..4 {
                    let col_w = columns[i].available_width();
                    columns[i].allocate_ui_with_layout(
                        egui::vec2(col_w, panel_h),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.set_width(col_w);
                            ui.set_min_width(col_w);
                            ui.set_max_width(col_w);
                            Self::show_column_title(ui, col_w, title_h, labels[i]);
                            ui.add_space(6.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w, img_h),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    ui.set_width(col_w);
                                    ui.set_max_width(col_w);
                                    ui.set_height(img_h);
                                    ui.set_max_height(img_h);
                                    if let Some(ref tex) = self.textures[i] {
                                        Self::show_image(ui, tex, self.img_width, self.img_height);
                                    }
                                },
                            );

                            ui.add_space(6.0);
                            if ui.button(&lbl_save_as).clicked() {
                                if let Some(img) = image::RgbaImage::from_raw(
                                    self.img_width,
                                    self.img_height,
                                    self.pixel_sets[i].to_vec(),
                                ) {
                                    let mut buf = Vec::new();
                                    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                                    if encoder
                                        .write_image(
                                            img.as_raw(),
                                            self.img_width,
                                            self.img_height,
                                            image::ColorType::Rgba8.into(),
                                        )
                                        .is_ok()
                                    {
                                        save_file_binary_async(
                                            &mut self.save_pending,
                                            &title,
                                            &filter_image,
                                            &["png"],
                                            &default_name,
                                            buf,
                                        );
                                    }
                                }
                            }
                        },
                    );
                }
            });
        }
    }
}
