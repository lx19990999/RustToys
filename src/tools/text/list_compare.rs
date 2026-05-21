use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use std::collections::HashSet;

pub struct ListComparer {
    list_a: String,
    list_b: String,
    result: String,
    case_sensitive: bool,
    trim_items: bool,
    ignore_empty: bool,
    comparison_mode: CompareMode,
    prev_a: String,
    prev_b: String,
    prev_mode: CompareMode,
    prev_case: bool,
    prev_trim: bool,
    prev_empty: bool,
    pending_file: Pending<String>,
    pending_open_target: Option<OpenListTarget>,
    save_pending: Pending<String>,
    save_result: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenListTarget {
    A,
    B,
}

#[derive(Clone, Copy, PartialEq)]
enum CompareMode {
    ItemsInBoth,
    OnlyInA,
    OnlyInB,
    AllDistinct,
    DistinctInA,
    DistinctInB,
}

impl CompareMode {
    fn label(self) -> String {
        match self {
            Self::ItemsInBoth => tr!("lc_both"),
            Self::OnlyInA => tr!("lc_only_a"),
            Self::OnlyInB => tr!("lc_only_b"),
            Self::AllDistinct => tr!("lc_all_distinct"),
            Self::DistinctInA => tr!("lc_distinct_a"),
            Self::DistinctInB => tr!("lc_distinct_b"),
        }
    }

    fn all() -> &'static [CompareMode] {
        &[
            Self::ItemsInBoth,
            Self::OnlyInA,
            Self::OnlyInB,
            Self::AllDistinct,
            Self::DistinctInA,
            Self::DistinctInB,
        ]
    }
}

impl Default for ListComparer {
    fn default() -> Self {
        Self {
            list_a: String::new(),
            list_b: String::new(),
            result: String::new(),
            case_sensitive: false,
            trim_items: true,
            ignore_empty: true,
            comparison_mode: CompareMode::ItemsInBoth,
            prev_a: String::new(),
            prev_b: String::new(),
            prev_mode: CompareMode::ItemsInBoth,
            prev_case: false,
            prev_trim: true,
            prev_empty: true,
            pending_file: Pending::default(),
            pending_open_target: None,
            save_pending: Pending::default(),
            save_result: String::new(),
        }
    }
}

impl Tool for ListComparer {
    fn name(&self) -> String { tr!("lc_name") }
    fn description(&self) -> String { tr!("lc_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Text }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let err_reading = tr!("err_error_reading");
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&err_reading) {
                match self.pending_open_target.take() {
                    Some(OpenListTarget::A) => self.list_a = text,
                    Some(OpenListTarget::B) => self.list_b = text,
                    None => {}
                }
            }
        }
        if let Some(text) = self.save_pending.poll() {
            self.save_result = text;
        }
        let total = ui.available_rect_before_wrap();
        let pad = 4.0;
        let w = total.width();
        let h = total.height();
        let half_w = (w - pad) * 0.5;

        // Layout sections - fixed heights for top/bottom, flexible middle
        let btn_row_h = 50.0;
        let options_h = 26.0;
        let result_label_h = 20.0;
        let result_btn_h = 26.0;
        let section_pad = pad;

        let fixed_h = btn_row_h + options_h + result_label_h + result_btn_h + section_pad * 3.0;
        let result_h = ((h - fixed_h) * 0.25).max(80.0).min(150.0);
        let cols_h = (h - fixed_h - result_h).max(100.0);

        // Auto-compare
        self.auto_compare();

        // --- Full-width button row ---
        let btn_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(w, btn_row_h),
        );
        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_paste_b = tr!("lc_paste_b");
        let lbl_open_b = tr!("lc_open_b");
        let lbl_copy_b = tr!("lc_copy_b");
        let lbl_save_b = tr!("lc_save_b");
        ui.scope_builder(egui::UiBuilder::new().max_rect(btn_rect), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_paste).clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.list_a = text,
                        Err(e) => self.result = tr!("err_clipboard", e),
                    }
                }
                if ui.button(&lbl_open).clicked() {
                    self.pending_open_target = Some(OpenListTarget::A);
                    open_file_async(&mut self.pending_file, &tr!("lc_open_a"), &tr!("save_filter_text"), &["txt"]);
                }
                    if ui.button(&lbl_clear).clicked() {
                    self.list_a.clear();
                    self.list_b.clear();
                    self.result.clear();
                }
                if ui.button(&lbl_copy).clicked() && !self.list_a.is_empty() {
                    ui.ctx().copy_text(self.list_a.clone());
                }
                if ui.button(&lbl_save_as).clicked() && !self.list_a.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("lc_save_a"), self.list_a.clone());
                }
                ui.separator();
                if ui.button(&lbl_paste_b).clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.list_b = text,
                        Err(e) => self.result = tr!("err_clipboard", e),
                    }
                }
                if ui.button(&lbl_open_b).clicked() {
                    self.pending_open_target = Some(OpenListTarget::B);
                    open_file_async(&mut self.pending_file, &tr!("lc_open_b"), &tr!("save_filter_text"), &["txt"]);
                }
                    if ui.button(&lbl_copy_b).clicked() && !self.list_b.is_empty() {
                    ui.ctx().copy_text(self.list_b.clone());
                }
                if ui.button(&lbl_save_b).clicked() && !self.list_b.is_empty() {
                    crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("lc_save_b_file"), self.list_b.clone());
                }
            });
        });

        // --- Two columns: List A and List B ---
        let cols_y = total.min.y + btn_row_h + section_pad;
        let lbl_list_a = tr!("lc_list_a");
        let lbl_list_b = tr!("lc_list_b");

        let left_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, cols_y),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_list_a).strong());
            ui.add_space(2.0);
            ui.add_sized(
                egui::vec2(half_w, (cols_h - 22.0).max(20.0)),
                egui::TextEdit::multiline(&mut self.list_a)
                    .font(egui::TextStyle::Monospace),
            );
        });

        let right_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x + half_w + pad, cols_y),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_list_b).strong());
            ui.add_space(2.0);
            ui.add_sized(
                egui::vec2(half_w, (cols_h - 22.0).max(20.0)),
                egui::TextEdit::multiline(&mut self.list_b)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Options row ---
        let options_y = cols_y + cols_h + section_pad;
        let options_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, options_y),
            egui::vec2(w, options_h),
        );
        let lbl_case_sensitive = tr!("lc_case_sensitive");
        let lbl_trim_items = tr!("lc_trim_items");
        let lbl_ignore_empty = tr!("lc_ignore_empty");
        ui.scope_builder(egui::UiBuilder::new().max_rect(options_rect), |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.case_sensitive, &lbl_case_sensitive);
                ui.checkbox(&mut self.trim_items, &lbl_trim_items);
                ui.checkbox(&mut self.ignore_empty, &lbl_ignore_empty);
                ui.separator();
                let selected_label = self.comparison_mode.label();
                egui::ComboBox::from_id_salt("compare_mode")
                    .selected_text(&selected_label)
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for &mode in CompareMode::all() {
                            let mode_label = mode.label();
                            ui.selectable_value(&mut self.comparison_mode, mode, &mode_label);
                        }
                    });
            });
        });

        // --- Result area ---
        let result_y = options_y + options_h + section_pad;
        let result_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, result_y),
            egui::vec2(w, result_h),
        );
        let lbl_result = tr!("lc_result");
        ui.scope_builder(egui::UiBuilder::new().max_rect(result_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_result).strong());
            let btn_h = if !self.result.is_empty() { 24.0 } else { 0.0 };
            let text_h = (result_h - result_label_h - btn_h - 2.0).max(20.0);
            ui.add_sized(
                egui::vec2(w, text_h),
                egui::TextEdit::multiline(&mut self.result)
                    .font(egui::TextStyle::Monospace),
            );
            if !self.result.is_empty() {
                ui.horizontal(|ui| {
                    if ui.button(&lbl_copy).clicked() {
                        ui.ctx().copy_text(self.result.clone());
                    }
                    if ui.button(&lbl_save_as).clicked() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("lc_save_result"), self.result.clone());
                    }
                });
            }
            if !self.save_result.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(0, 180, 0), &self.save_result);
            }
        });
    }
}

impl ListComparer {
    fn auto_compare(&mut self) {
        if self.list_a != self.prev_a
            || self.list_b != self.prev_b
            || self.comparison_mode != self.prev_mode
            || self.case_sensitive != self.prev_case
            || self.trim_items != self.prev_trim
            || self.ignore_empty != self.prev_empty
        {
            self.prev_a = self.list_a.clone();
            self.prev_b = self.list_b.clone();
            self.prev_mode = self.comparison_mode;
            self.prev_case = self.case_sensitive;
            self.prev_trim = self.trim_items;
            self.prev_empty = self.ignore_empty;
            self.do_compare();
        }
    }

    fn do_compare(&mut self) {
        if self.list_a.trim().is_empty() && self.list_b.trim().is_empty() {
            self.result.clear();
            return;
        }

        let set_a = self.build_set(&self.list_a);
        let set_b = self.build_set(&self.list_b);

        let items: Vec<String> = match self.comparison_mode {
            CompareMode::ItemsInBoth => set_a.intersection(&set_b).cloned().collect(),
            CompareMode::OnlyInA => set_a.difference(&set_b).cloned().collect(),
            CompareMode::OnlyInB => set_b.difference(&set_a).cloned().collect(),
            CompareMode::AllDistinct => set_a.union(&set_b).cloned().collect(),
            CompareMode::DistinctInA => {
                let a_only: HashSet<_> = set_a.difference(&set_b).cloned().collect();
                let inter: HashSet<_> = set_a.intersection(&set_b).cloned().collect();
                a_only.into_iter().chain(inter).collect()
            }
            CompareMode::DistinctInB => {
                let b_only: HashSet<_> = set_b.difference(&set_a).cloned().collect();
                let inter: HashSet<_> = set_b.intersection(&set_a).cloned().collect();
                b_only.into_iter().chain(inter).collect()
            }
        };

        let mut sorted: Vec<String> = items;
        sorted.sort();
        self.result = sorted.join("\n");
    }

    fn build_set(&self, text: &str) -> HashSet<String> {
        let mut set = HashSet::new();
        for line in text.lines() {
            let mut item = if self.trim_items { line.trim().to_string() } else { line.to_string() };
            if self.ignore_empty && item.is_empty() {
                continue;
            }
            if !self.case_sensitive {
                item = item.to_lowercase();
            }
            set.insert(item);
        }
        set
    }
}
