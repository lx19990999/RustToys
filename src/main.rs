#![windows_subsystem = "windows"]

mod app;
mod config;
mod i18n;
mod sidebar;
mod tool;
mod tools;

fn main() -> eframe::Result {
    config::init();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("RustToys"),
        ..Default::default()
    };

    eframe::run_native(
        "RustToys",
        native_options,
        Box::new(|cc| {
            install_cjk_font(&cc.egui_ctx);
            Ok(Box::new(app::RustToysApp::new(cc)))
        }),
    )
}

fn install_cjk_font(ctx: &eframe::egui::Context) {
    use eframe::egui::text::FontData;
    use eframe::egui::epaint::text::{FontInsert, InsertFontFamily, FontPriority};
    use eframe::egui::FontFamily;

    const CJK_FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSansSC-Regular.ttf");

    ctx.add_font(FontInsert::new(
        "NotoSansSC",
        FontData::from_static(CJK_FONT_BYTES),
        vec![
            InsertFontFamily { family: FontFamily::Proportional, priority: FontPriority::Lowest },
            InsertFontFamily { family: FontFamily::Monospace, priority: FontPriority::Lowest },
        ],
    ));
}
