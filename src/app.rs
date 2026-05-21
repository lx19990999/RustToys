use eframe::egui;
use crate::sidebar::Sidebar;
use crate::tool::Tool;
use crate::tools;
use crate::config;
use crate::i18n::Language;
use crate::tr;

pub struct RustToysApp {
    sidebar: Sidebar,
    tools: Vec<Box<dyn Tool>>,
    dpi_scale: f32,
    dpi_initialized: bool,
    theme_mode: config::ThemeMode,
    language: Language,
}

impl RustToysApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = config::get();
        let (dpi_scale, dpi_initialized) = if cfg.dpi > 0.0 {
            (cfg.dpi, true)
        } else {
            (1.0, false)
        };
        Self {
            sidebar: Sidebar::default(),
            tools: tools::all_tools(),
            dpi_scale,
            dpi_initialized,
            theme_mode: cfg.theme,
            language: cfg.language,
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        match self.theme_mode {
            config::ThemeMode::Light => ctx.set_visuals(egui::Visuals::light()),
            config::ThemeMode::Dark => ctx.set_visuals(egui::Visuals::dark()),
            config::ThemeMode::System => {
                let visuals = match dark_light::detect() {
                    dark_light::Mode::Dark => egui::Visuals::dark(),
                    _ => egui::Visuals::light(),
                };
                ctx.set_visuals(visuals);
            }
        }
    }
}

impl eframe::App for RustToysApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Auto-detect high-res monitor on first frame
        if !self.dpi_initialized {
            self.dpi_initialized = true;
            ctx.input(|i| {
                let vp = i.raw.viewport();
                let native_ppp = vp.native_pixels_per_point.unwrap_or(1.0);
                let monitor_pts = vp.monitor_size.unwrap_or(egui::vec2(1920.0, 1080.0));
                let pixel_w = monitor_pts.x * native_ppp;
                let pixel_h = monitor_pts.y * native_ppp;
                if pixel_w > 2560.0 || pixel_h > 1440.0 {
                    self.dpi_scale = 2.0;
                } else {
                    self.dpi_scale = native_ppp.max(1.0);
                }
            });
            config::update(|cfg| cfg.dpi = self.dpi_scale);
            ctx.set_pixels_per_point(self.dpi_scale);
        }

        // Apply DPI scale
        ctx.set_pixels_per_point(self.dpi_scale);

        // Apply theme
        self.apply_theme(ctx);

        let ppp = self.dpi_scale;

        // Top panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.heading(tr!("app_title"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // DPI control
                    ui.label(tr!("dpi_label", self.dpi_scale));
                    if ui.button("-").clicked() && self.dpi_scale > 0.5 {
                        self.dpi_scale = (self.dpi_scale - 0.25).max(0.5);
                        config::update(|cfg| cfg.dpi = self.dpi_scale);
                    }
                    if ui.button("+").clicked() && self.dpi_scale < 4.0 {
                        self.dpi_scale = (self.dpi_scale + 0.25).min(4.0);
                        config::update(|cfg| cfg.dpi = self.dpi_scale);
                    }
                    ui.separator();

                    // Language selector
                    let prev_lang = self.language;
                    egui::ComboBox::from_id_salt("language_selector")
                        .selected_text(self.language.label())
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for &lang in Language::all() {
                                ui.selectable_value(&mut self.language, lang, lang.label());
                            }
                        });
                    if self.language != prev_lang {
                        crate::i18n::set(self.language);
                        config::update(|cfg| cfg.language = self.language);
                    }
                    ui.separator();

                    // Theme selector
                    let prev_theme = self.theme_mode;
                    let theme_label = tr!("theme_label", self.theme_mode.label());
                    egui::ComboBox::from_id_salt("theme_selector")
                        .selected_text(theme_label)
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.theme_mode, config::ThemeMode::Light, tr!("theme_light"));
                            ui.selectable_value(&mut self.theme_mode, config::ThemeMode::Dark, tr!("theme_dark"));
                            ui.selectable_value(&mut self.theme_mode, config::ThemeMode::System, tr!("theme_system"));
                        });
                    if self.theme_mode != prev_theme {
                        config::update(|cfg| cfg.theme = self.theme_mode);
                    }
                    ui.separator();
                    if ui.button(tr!("quit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        // Left side panel
        egui::SidePanel::left("side_panel")
            .default_width(220.0 * ppp.max(1.0))
            .resizable(true)
            .show(ctx, |ui| {
                self.sidebar.show(ui, &mut self.tools);
            });

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let idx = self.sidebar.selected_tool;
            if idx < self.tools.len() {
                let tool = &mut self.tools[idx];
                ui.heading(tool.name());
                ui.label(tool.description());
                ui.separator();
                ui.add_space(8.0);
                tool.ui(ui);
            }
        });

        // Bottom panel
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr!("tools_count", self.tools.len()));
                ui.separator();
                let idx = self.sidebar.selected_tool;
                if idx < self.tools.len() {
                    ui.label(tr!("active_label", self.tools[idx].name()));
                }
            });
        });
    }
}
