//! Two-column input/output layout with fixed 50/50 column widths.

use eframe::egui;

const COL_GAP: f32 = 12.0;
pub const ROW_GAP: f32 = 4.0;

pub enum IoColumn {
    Left,
    Right,
}

/// Width of each column (half of available space minus gap).
pub fn half_column_width(ui: &egui::Ui) -> f32 {
    ((ui.available_width() - COL_GAP) * 0.5).max(120.0)
}

/// Standard height for an empty row that mirrors a checkbox/options row on the other side.
pub fn option_row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y + ROW_GAP
}

pub fn show_error(ui: &mut egui::Ui, error: &str) {
    if !error.is_empty() {
        ui.colored_label(egui::Color32::RED, error);
        ui.add_space(ROW_GAP);
    }
}

/// Reserved error line(s) so showing/hiding messages does not shift widgets or steal focus.
pub fn error_slot(ui: &mut egui::Ui, message: &str, lines: usize) {
    let line_h = ui.text_style_height(&egui::TextStyle::Body);
    let h = (line_h * lines as f32).max(line_h);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            if !message.is_empty() {
                ui.colored_label(egui::Color32::RED, message);
            }
        },
    );
}

pub fn row_spacer(ui: &mut egui::Ui, height: f32) {
    ui.allocate_space(egui::vec2(0.0, height));
}

/// Heights for two-column I/O: title row + toolbar row + gaps, then scroll field.
pub fn aligned_io_heights(ui: &egui::Ui) -> (f32, f32, f32) {
    let opt_h = option_row_height(ui);
    let body_h = ui.available_height().max(120.0);
    let column_top_h = opt_h + opt_h + ROW_GAP * 2.0;
    let field_h = (body_h - column_top_h).max(40.0);
    (opt_h, body_h, field_h)
}

/// Fixed-height column title row so left/right scroll areas align at the top.
pub fn column_header_row(ui: &mut egui::Ui, col_w: f32, h: f32, add_header: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(col_w, h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(col_w);
            ui.set_height(h);
            ui.set_min_height(h);
            ui.set_max_height(h);
            add_header(ui);
        },
    );
}

/// Fixed-min-height toolbar row (supports wrapped buttons without shifting the peer column).
pub fn toolbar_row(ui: &mut egui::Ui, col_w: f32, min_h: f32, add_toolbar: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(col_w, min_h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(col_w);
            ui.set_min_height(min_h);
            add_toolbar(ui);
        },
    );
}

/// Scrollable multiline editor with explicit height (for aligned two-column I/O).
pub fn multiline_field_at(
    ui: &mut egui::Ui,
    col_w: f32,
    h: f32,
    scroll_id: impl std::hash::Hash,
    text: &mut String,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(col_w, h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(col_w);
            ui.set_max_width(col_w);
            ui.set_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(col_w);
                    ui.set_max_width(col_w);
                    ui.add(
                        egui::TextEdit::multiline(text)
                            .desired_width(col_w)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        },
    );
}

/// Scrollable multiline editor using remaining column height.
pub fn multiline_field(
    ui: &mut egui::Ui,
    col_w: f32,
    scroll_id: impl std::hash::Hash,
    text: &mut String,
) {
    let h = ui.available_height().max(40.0);
    ui.allocate_ui_with_layout(
        egui::vec2(col_w, h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.set_width(col_w);
            ui.set_max_width(col_w);
            ui.set_height(h);
            ui.set_max_height(h);
            egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(col_w);
                    ui.set_max_width(col_w);
                    ui.add(
                        egui::TextEdit::multiline(text)
                            .desired_width(col_w)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        },
    );
}

/// Alias for network tools (same scrollable field).
pub fn multiline_scroll_field(
    ui: &mut egui::Ui,
    col_w: f32,
    scroll_id: impl std::hash::Hash,
    text: &mut String,
) {
    multiline_field(ui, col_w, scroll_id, text);
}

fn allocate_column(ui: &mut egui::Ui, w: f32, h: f32, f: &mut impl FnMut(&mut egui::Ui, f32)) {
    ui.allocate_ui_with_layout(
        egui::vec2(w, h),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            let rect = ui.max_rect();
            ui.set_clip_rect(rect);
            ui.set_width(w);
            ui.set_min_width(w);
            ui.set_max_width(w);
            ui.set_height(h);
            ui.set_max_height(h);
            f(ui, w);
        },
    );
}

fn two_column_row<F>(ui: &mut egui::Ui, body_h: f32, mut f: F)
where
    F: FnMut(&mut egui::Ui, f32, IoColumn),
{
    let half_w = half_column_width(ui);
    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        allocate_column(ui, half_w, body_h, &mut |ui, w| f(ui, w, IoColumn::Left));
        ui.add_space(COL_GAP);
        allocate_column(ui, half_w, body_h, &mut |ui, w| f(ui, w, IoColumn::Right));
    });
}

/// Fixed 50/50 columns. Call [`show_error`] before this if needed.
pub fn two_column_io<F>(ui: &mut egui::Ui, f: F)
where
    F: FnMut(&mut egui::Ui, f32, IoColumn),
{
    let body_h = ui.available_height().max(80.0);
    two_column_io_with_height(ui, body_h, f);
}

pub fn two_column_io_with_height<F>(ui: &mut egui::Ui, body_h: f32, f: F)
where
    F: FnMut(&mut egui::Ui, f32, IoColumn),
{
    two_column_row(ui, body_h, f);
}

/// Network tools: same fixed 50/50 columns with scrollable text areas.
pub fn two_column_scroll_io<F>(ui: &mut egui::Ui, f: F)
where
    F: FnMut(&mut egui::Ui, f32, IoColumn),
{
    let body_h = ui.available_height().max(80.0);
    two_column_row(ui, body_h, f);
}
