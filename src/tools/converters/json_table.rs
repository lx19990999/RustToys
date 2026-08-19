use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async, save_file_async};
use serde_json::Value;
use std::collections::BTreeMap;

const MIN_COL_WIDTH: f32 = 80.0;
const MAX_COL_WIDTH: f32 = 400.0;
const COL_PADDING: f32 = 16.0;


pub struct JsonTable {
    input: String,
    error: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    col_widths: Vec<f32>,
    sort_alpha: bool,
    flatten_nested: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for JsonTable {
    fn default() -> Self {
        Self {
            input: String::new(),
            error: String::new(),
            headers: Vec::new(),
            rows: Vec::new(),
            col_widths: Vec::new(),
            sort_alpha: false,
            flatten_nested: false,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}


impl Tool for JsonTable {
    fn name(&self) -> String { tr!("jt_name") }
    fn description(&self) -> String { tr!("jt_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Converters }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_flatten = self.flatten_nested;
        let prev_sort = self.sort_alpha;

        if let Some(path) = crate::tools::async_utils::take_dropped_file(ui.ctx()) {
            crate::tools::async_utils::open_dropped_text_async(&mut self.pending_file, path);
        }
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&tr!("err_error_reading")) {
                self.input = text;
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.error = text;
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.flatten_nested, tr!("jt_flatten"));
            ui.checkbox(&mut self.sort_alpha, tr!("jt_sort_columns"));
        });
        ui.add_space(4.0);

        // Left-right layout: input | output
        ui.columns(2, |cols| {
            // Left: Input panel
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match crate::clipboard::read_text() {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("save_as_title"), "JSON", &["json"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.headers.clear();
                        self.rows.clear();
                        self.col_widths.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("jt_input_label"));

                // ScrollArea provides mouse-wheel scrolling.
                // TextEdit uses scroll_to_rect() to keep cursor in view — requires parent ScrollArea.
                // No conflict: ScrollArea handles wheel, TextEdit handles click/drag selection.
                egui::ScrollArea::vertical()
                    .id_salt("json_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output panel
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                if self.headers.is_empty() {
                    ui.label(egui::RichText::new(tr!("jt_placeholder")).italics().weak());
                    return;
                }

                ui.label(tr!("jt_rows_cols", self.rows.len(), self.headers.len()));

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy_csv")).clicked() {
                        ui.ctx().copy_text(self.to_csv(","));
                    }
                    if ui.button(tr!("btn_copy_tsv")).clicked() {
                        ui.ctx().copy_text(self.to_csv("\t"));
                    }
                    if ui.button(tr!("btn_copy_markdown")).clicked() {
                        ui.ctx().copy_text(self.to_markdown());
                    }
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_save_csv")).clicked() {
                        self.save_as_file("csv", self.to_csv(","));
                    }
                    if ui.button(tr!("btn_save_tsv")).clicked() {
                        self.save_as_file("tsv", self.to_csv("\t"));
                    }
                    if ui.button(tr!("btn_save_json")).clicked() {
                        self.save_as_file("json", self.input.clone());
                    }
                });
                ui.add_space(4.0);

                // Recompute column widths from actual rendered text metrics
                self.compute_col_widths(ui);

                // Table with manual layout — ScrollArea::both() for vertical + horizontal scroll
                egui::ScrollArea::both()
                    .id_salt("json_table_scroll")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height() - 20.0)
                    .show(ui, |ui| {
                        let style = ui.style().clone();
                        let row_bg = style.visuals.faint_bg_color;
                        let text_color = style.visuals.text_color();
                        let header_color = style.visuals.strong_text_color();
                        let stripe_color = style.visuals.window_fill;
                        let row_h = 20.0;

                        // Header row
                        ui.horizontal(|ui| {
                            for (ci, h) in self.headers.iter().enumerate() {
                                let w = self.col_widths.get(ci).copied().unwrap_or(MIN_COL_WIDTH);
                                let rect = egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(w, row_h),
                                );
                                ui.painter().rect_filled(rect, 0.0, row_bg);
                                ui.painter().text(
                                    rect.min + egui::vec2(4.0, row_h * 0.5),
                                    egui::Align2::LEFT_CENTER,
                                    h,
                                    egui::FontId::proportional(13.0),
                                    header_color,
                                );
                                ui.advance_cursor_after_rect(rect);
                            }
                        });
                        ui.separator();

                        // Data rows
                        for (ri, row) in self.rows.iter().enumerate() {
                            let bg = if ri % 2 == 0 { row_bg } else { stripe_color };
                            ui.horizontal(|ui| {
                                for (ci, cell) in row.iter().enumerate() {
                                    let w = self.col_widths.get(ci).copied().unwrap_or(MIN_COL_WIDTH);
                                    let max_chars = ((w / 7.0) as usize).max(4);
                                    let display = if cell.len() > max_chars {
                                        format!("{}...", &cell[..max_chars - 3])
                                    } else {
                                        cell.clone()
                                    };
                                    let rect = egui::Rect::from_min_size(
                                        ui.cursor().min,
                                        egui::vec2(w, row_h),
                                    );
                                    ui.painter().rect_filled(rect, 0.0, bg);
                                    ui.painter().text(
                                        rect.min + egui::vec2(4.0, row_h * 0.5),
                                        egui::Align2::LEFT_CENTER,
                                        &display,
                                        egui::FontId::monospace(12.0),
                                        text_color,
                                    );
                                    ui.advance_cursor_after_rect(rect);
                                }
                            });
                        }
                    });
            });
        });

        // Auto-convert when input or options change
        if self.input != prev_input || self.flatten_nested != prev_flatten || self.sort_alpha != prev_sort {
            if !self.input.trim().is_empty() {
                self.convert();
            } else {
                self.headers.clear();
                self.rows.clear();
                self.error.clear();
            }
        }
    }
}

impl JsonTable {
    fn compute_col_widths(&mut self, ui: &egui::Ui) {
        let fonts = ui.fonts(|f| f.clone());
        let n = self.headers.len();
        self.col_widths = vec![MIN_COL_WIDTH; n];

        // Measure headers
        for (ci, h) in self.headers.iter().enumerate() {
            let galley = fonts.layout_no_wrap(
                h.clone(),
                egui::FontId::proportional(13.0),
                egui::Color32::WHITE,
            );
            self.col_widths[ci] = self.col_widths[ci].max(galley.size().x + COL_PADDING);
        }

        // Sample up to 100 rows to estimate column widths
        let sample_count = self.rows.len().min(100);
        for row in self.rows.iter().take(sample_count) {
            for (ci, cell) in row.iter().enumerate() {
                if ci < n {
                    let galley = fonts.layout_no_wrap(
                        cell.clone(),
                        egui::FontId::monospace(12.0),
                        egui::Color32::WHITE,
                    );
                    self.col_widths[ci] = self.col_widths[ci]
                        .max(galley.size().x + COL_PADDING)
                        .min(MAX_COL_WIDTH);
                }
            }
        }
    }

    fn convert(&mut self) {
        self.error.clear();
        self.headers.clear();
        self.rows.clear();
        self.col_widths.clear();

        let parsed: Value = match serde_json::from_str(&self.input) {
            Ok(v) => v,
            Err(e) => {
                self.error = tr!("jt_json_parse_error", e);
                return;
            }
        };

        let arr = match parsed {
            Value::Array(a) => a,
            _ => {
                self.error = tr!("jt_need_array");
                return;
            }
        };

        if arr.is_empty() {
            self.error = tr!("jt_empty_array");
            return;
        }

        let mut all_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut flat_rows: Vec<BTreeMap<String, String>> = Vec::new();

        for item in &arr {
            let obj = match item {
                Value::Object(o) => o,
                _ => {
                    flat_rows.push([(tr!("jt_value"), json_to_string(item))].into_iter().collect());
                    all_keys.insert(tr!("jt_value"));
                    continue;
                }
            };

            if self.flatten_nested {
                let flat = flatten_json(obj, "");
                for k in flat.keys() {
                    all_keys.insert(k.clone());
                }
                flat_rows.push(flat);
            } else {
                let mut row = BTreeMap::new();
                for (k, v) in obj {
                    row.insert(k.clone(), json_to_string(v));
                    all_keys.insert(k.clone());
                }
                flat_rows.push(row);
            }
        }

        self.headers = if self.sort_alpha {
            all_keys.into_iter().collect()
        } else {
            let first_keys: Vec<String> = if let Some(first) = flat_rows.first() {
                first.keys().cloned().collect()
            } else {
                vec![]
            };
            let mut remaining: Vec<String> = all_keys.into_iter()
                .filter(|k| !first_keys.contains(k))
                .collect();
            let mut result = first_keys;
            result.append(&mut remaining);
            result
        };

        for flat in &flat_rows {
            let row: Vec<String> = self.headers.iter()
                .map(|h| flat.get(h).cloned().unwrap_or_default())
                .collect();
            self.rows.push(row);
        }
    }

    fn to_csv(&self, sep: &str) -> String {
        let mut lines = Vec::new();
        let header: Vec<String> = self.headers.iter()
            .map(|h| csv_escape(h, sep))
            .collect();
        lines.push(header.join(sep));
        for row in &self.rows {
            let cells: Vec<String> = row.iter()
                .map(|c| csv_escape(c, sep))
                .collect();
            lines.push(cells.join(sep));
        }
        lines.join("\n")
    }

    fn to_markdown(&self) -> String {
        let mut lines = Vec::new();
        let header: Vec<String> = self.headers.iter().map(|h| format!(" {} ", h)).collect();
        lines.push(format!("|{}|", header.join("|")));
        let sep: Vec<String> = self.headers.iter().map(|_| "---".to_string()).collect();
        lines.push(format!("|{}|", sep.join("|")));
        for row in &self.rows {
            let cells: Vec<String> = row.iter().map(|c| format!(" {} ", c)).collect();
            lines.push(format!("|{}|", cells.join("|")));
        }
        lines.join("\n")
    }

    fn save_as_file(&mut self, ext: &str, content: String) {
        save_file_async(
            &mut self.save_pending,
            &tr!("jt_save_as", ext.to_uppercase()),
            &ext.to_uppercase(),
            &[ext],
            &format!("table.{}", ext),
            content,
        );
    }
}

fn json_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

fn flatten_json(obj: &serde_json::Map<String, Value>, prefix: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for (key, value) in obj {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            Value::Object(nested) => {
                let sub = flatten_json(nested, &full_key);
                result.extend(sub);
            }
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| json_to_string(v)).collect();
                result.insert(full_key, items.join(", "));
            }
            _ => {
                result.insert(full_key, json_to_string(value));
            }
        }
    }
    result
}

fn csv_escape(field: &str, sep: &str) -> String {
    if field.contains(sep) || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}
