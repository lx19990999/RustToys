#![windows_subsystem = "windows"]

mod app;
mod autostart;
mod clipboard;
mod config;
mod i18n;
mod sidebar;
mod tool;
mod tools;

use raw_window_handle::HasDisplayHandle;

fn main() -> eframe::Result {
    config::init();

    let cfg = config::get();
    if cfg.autostart {
        let _ = autostart::set(true);
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("RustToys")
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "RustToys",
        native_options,
        Box::new(|cc| {
            if let Ok(handle) = cc.display_handle() {
                clipboard::init(Some(handle.as_raw()));
            }
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

fn load_icon() -> eframe::egui::IconData {
    const ICON_BYTES: &[u8] = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(ICON_BYTES)
        .expect("Failed to load icon")
        .to_rgba8();
    let (width, height) = img.dimensions();
    eframe::egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}
