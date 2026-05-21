use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct MarkdownPreview {
    input: String,
    html_output: String,
    theme: usize, // 0=GitHub Light, 1=GitHub Dark
    pending_file: Pending<String>,
    save_pending: Pending<String>,
    save_result: String,
}

impl Default for MarkdownPreview {
    fn default() -> Self {
        Self {
            input: String::new(),
            html_output: String::new(),
            theme: 0,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
            save_result: String::new(),
        }
    }
}

impl Tool for MarkdownPreview {
    fn name(&self) -> String { tr!("md_name") }
    fn description(&self) -> String { tr!("md_desc") }
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
        let top_h = label_h + space + btn_h * 2.0 + space * 3.0;
        let theme_h = 22.0 + space;

        let cols_h = (h - theme_h - pad * 2.0).max(120.0);

        // Generate HTML for output
        self.update_html();

        // --- Theme selector ---
        let theme_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(w, theme_h),
        );
        let lbl_theme = tr!("md_theme_label");
        let lbl_github_light = tr!("md_github_light");
        let lbl_github_dark = tr!("md_github_dark");
        ui.scope_builder(egui::UiBuilder::new().max_rect(theme_rect), |ui| {
            ui.horizontal(|ui| {
                ui.label(&lbl_theme);
                ui.radio_value(&mut self.theme, 0, &lbl_github_light);
                ui.radio_value(&mut self.theme, 1, &lbl_github_dark);
            });
        });

        let cols_y = total.min.y + theme_h + pad;

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
        let lbl_markdown = tr!("md_markdown_label");
        let lbl_preview = tr!("md_preview_label");
        let lbl_copy_html = tr!("btn_copy_html");
        let lbl_save_html = tr!("btn_save_html");
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_markdown).strong());
            ui.add_space(space);
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_paste).clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.input = text,
                        Err(e) => self.html_output = tr!("err_clipboard", e),
                    }
                }
                if ui.button(&lbl_open).clicked() {
                    open_file_async(&mut self.pending_file, &tr!("md_open_title"), &tr!("md_filter_md"), &["md", "markdown"]);
                }
                    if ui.button(&lbl_clear).clicked() {
                    self.input.clear();
                }
                if ui.button(&lbl_copy).clicked() && !self.input.is_empty() {
                    ui.ctx().copy_text(self.input.clone());
                }
                if ui.button(&lbl_save_as).clicked() && !self.input.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("md_filter_md"), &["md"], &tr!("md_save_default"), self.input.clone());
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

        // --- Right: Preview ---
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x + half_w + pad, cols_y),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_preview).strong());
            ui.add_space(space);
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_copy_html).clicked() && !self.html_output.is_empty() {
                    ui.ctx().copy_text(self.html_output.clone());
                }
                if ui.button(&lbl_save_html).clicked() && !self.html_output.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("md_filter_html"), &["html"], &tr!("md_save_html"), self.html_output.clone());
                }
                if !self.save_result.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.save_result);
                }
            });
            ui.add_space(space);
            let preview_h = (cols_h - top_h).max(40.0);
            let is_dark = self.theme == 1;
            egui::ScrollArea::vertical()
                .id_salt("md_preview_scroll")
                .max_height(preview_h)
                .show(ui, |ui| {
                    self.render_content(ui, is_dark);
                });
        });
    }
}

impl MarkdownPreview {
    fn update_html(&mut self) {
        let parser = pulldown_cmark::Parser::new(&self.input);
        let mut html_buf = String::new();
        pulldown_cmark::html::push_html(&mut html_buf, parser);
        self.html_output = html_buf;
    }

    fn render_content(&self, ui: &mut egui::Ui, is_dark: bool) {
        let heading_color = if is_dark { egui::Color32::WHITE } else { egui::Color32::BLACK };
        let body_color = if is_dark { egui::Color32::from_rgb(200, 200, 200) } else { egui::Color32::from_rgb(50, 50, 50) };
        let code_bg = if is_dark { egui::Color32::from_rgb(40, 44, 52) } else { egui::Color32::from_rgb(246, 248, 250) };
        let quote_color = if is_dark { egui::Color32::from_rgb(120, 130, 140) } else { egui::Color32::from_rgb(100, 110, 120) };

        let mut in_code_block = false;
        let mut code_buf = String::new();

        let lines: Vec<&str> = self.input.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();

            if trimmed.starts_with("```") {
                if in_code_block {
                    in_code_block = false;
                    let code_style = egui::RichText::new(&code_buf)
                        .monospace()
                        .color(body_color)
                        .background_color(code_bg);
                    ui.label(code_style);
                    code_buf.clear();
                } else {
                    in_code_block = true;
                }
                i += 1;
                continue;
            }

            if in_code_block {
                code_buf.push_str(lines[i]);
                code_buf.push('\n');
                i += 1;
                continue;
            }

            // Check for table: line with | and next line is separator
            if trimmed.contains('|') && i + 1 < lines.len() {
                let next_trimmed = lines[i + 1].trim();
                if is_table_separator(next_trimmed) {
                    // Collect table rows
                    let header_cells = parse_table_row(trimmed);
                    let ncols = header_cells.len();
                    i += 2; // skip header and separator

                    let mut data_rows: Vec<Vec<String>> = Vec::new();
                    while i < lines.len() {
                        let row_trimmed = lines[i].trim();
                        if row_trimmed.contains('|') && !row_trimmed.is_empty() {
                            data_rows.push(parse_table_row(row_trimmed));
                            i += 1;
                        } else {
                            break;
                        }
                    }

                    // Render table
                    ui.add_space(4.0);
                    egui::Grid::new(format!("md_table_{}", i))
                        .num_columns(ncols)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header row
                            for cell in &header_cells {
                                ui.label(egui::RichText::new(cell.as_str()).strong().color(heading_color));
                            }
                            ui.end_row();
                            // Data rows
                            for row in &data_rows {
                                for (ci, cell) in row.iter().enumerate() {
                                    if ci < ncols {
                                        ui.label(egui::RichText::new(cell.as_str()).color(body_color));
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(4.0);
                    continue;
                }
            }

            if trimmed.starts_with("# ") {
                ui.label(egui::RichText::new(&trimmed[2..]).heading().color(heading_color).size(28.0));
            } else if trimmed.starts_with("## ") {
                ui.label(egui::RichText::new(&trimmed[3..]).strong().color(heading_color).size(22.0));
            } else if trimmed.starts_with("### ") {
                ui.label(egui::RichText::new(&trimmed[4..]).strong().color(heading_color).size(18.0));
            } else if trimmed.starts_with("#### ") {
                ui.label(egui::RichText::new(&trimmed[5..]).strong().color(heading_color).size(16.0));
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                ui.label(egui::RichText::new(format!("  • {}", &trimmed[2..])).color(body_color));
            } else if trimmed.starts_with("> ") {
                ui.label(egui::RichText::new(format!("│ {}", &trimmed[2..])).italics().color(quote_color));
            } else if trimmed.starts_with("---") || trimmed.starts_with("***") {
                ui.separator();
            } else if trimmed.starts_with("`") && trimmed.ends_with("`") && trimmed.len() > 2 {
                ui.label(egui::RichText::new(&trimmed[1..trimmed.len()-1]).monospace().background_color(code_bg));
            } else if !trimmed.is_empty() {
                ui.label(egui::RichText::new(trimmed).color(body_color));
            } else {
                ui.add_space(4.0);
            }

            i += 1;
        }
    }
}

fn is_table_separator(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || !t.contains('|') { return false; }
    t.replace('|', "").replace('-', "").replace(':', "").trim().is_empty()
        && t.contains('-')
}

fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = if trimmed.starts_with('|') { &trimmed[1..] } else { trimmed };
    let inner = if inner.ends_with('|') { &inner[..inner.len()-1] } else { inner };
    inner.split('|').map(|c| c.trim().to_string()).collect()
}
