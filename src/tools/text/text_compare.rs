use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct TextComparer {
    text_a: String,
    text_b: String,
    diff_result: Vec<DiffLine>,
    prev_a: String,
    prev_b: String,
    pending_file: Pending<String>,
    pending_diff: Pending<Vec<DiffLine>>,
}

struct DiffLine {
    line: String,
    kind: DiffKind,
}

#[derive(Clone, Copy, PartialEq)]
enum DiffKind {
    Equal,
    Added,
    Removed,
}

impl Default for DiffKind {
    fn default() -> Self { DiffKind::Equal }
}

impl Default for TextComparer {
    fn default() -> Self {
        Self {
            text_a: String::new(),
            text_b: String::new(),
            diff_result: Vec::new(),
            prev_a: String::new(),
            prev_b: String::new(),
            pending_file: Pending::default(),
            pending_diff: Pending::default(),
        }
    }
}

impl Tool for TextComparer {
    fn name(&self) -> &str { "Text Comparer" }
    fn description(&self) -> &str { "Compare two texts and highlight differences" }
    fn category(&self) -> ToolCategory { ToolCategory::Text }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // Poll async file read results
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                if self.text_a.is_empty() {
                    self.text_a = text;
                } else {
                    self.text_b = text;
                }
            }
        }

        // Poll async diff results
        if let Some(diff) = self.pending_diff.poll() {
            self.diff_result = diff;
            self.prev_a = self.text_a.clone();
            self.prev_b = self.text_b.clone();
        }

        // Trigger async diff when input changes (only if not already computing)
        if !self.pending_diff.is_pending() && (self.text_a != self.prev_a || self.text_b != self.prev_b) {
            if !self.text_a.is_empty() || !self.text_b.is_empty() {
                let a = self.text_a.clone();
                let b = self.text_b.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                self.pending_diff.set_receiver(rx);
                std::thread::spawn(move || {
                    let result = compute_diff(&a, &b);
                    let _ = tx.send(result);
                });
            } else {
                self.diff_result.clear();
                self.prev_a.clear();
                self.prev_b.clear();
            }
        }

        // Input columns
        ui.columns(2, |cols| {
            // --- Text A ---
            cols[0].label(egui::RichText::new("Text A").strong());
            cols[0].horizontal_wrapped(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.text_a = text,
                        Err(e) => { self.diff_result = vec![DiffLine { line: format!("Clipboard error: {}", e), kind: DiffKind::Removed }]; }
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open text A", "Text", &["txt"]);
                }
                if ui.button("Clear").clicked() {
                    self.text_a.clear();
                    self.diff_result.clear();
                }
                if ui.button("Copy").clicked() && !self.text_a.is_empty() {
                    ui.ctx().copy_text(self.text_a.clone());
                }
                if ui.button("Save As...").clicked() && !self.text_a.is_empty() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "text_a.txt") {
                        let _ = std::fs::write(path, &self.text_a);
                    }
                }
            });
            cols[0].add_space(2.0);
            egui::ScrollArea::both()
                .id_salt("text_a_scroll")
                .max_height(200.0)
                .show(&mut cols[0], |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text_a)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Monospace),
                    );
                });

            // --- Text B ---
            cols[1].label(egui::RichText::new("Text B").strong());
            cols[1].horizontal_wrapped(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => self.text_b = text,
                        Err(e) => { self.diff_result = vec![DiffLine { line: format!("Clipboard error: {}", e), kind: DiffKind::Added }]; }
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open text B", "Text", &["txt"]);
                }
                if ui.button("Clear").clicked() {
                    self.text_b.clear();
                    self.diff_result.clear();
                }
                if ui.button("Copy").clicked() && !self.text_b.is_empty() {
                    ui.ctx().copy_text(self.text_b.clone());
                }
                if ui.button("Save As...").clicked() && !self.text_b.is_empty() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "text_b.txt") {
                        let _ = std::fs::write(path, &self.text_b);
                    }
                }
            });
            cols[1].add_space(2.0);
            egui::ScrollArea::both()
                .id_salt("text_b_scroll")
                .max_height(200.0)
                .show(&mut cols[1], |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.text_b)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Monospace),
                    );
                });
        });

        ui.add_space(8.0);

        // Show computing indicator
        if self.pending_diff.is_pending() {
            ui.label(egui::RichText::new("Computing diff...").italics().color(egui::Color32::from_rgb(100, 100, 200)));
        }

        // --- Diff Result ---
        ui.label(egui::RichText::new("Diff Result").strong());
        ui.add_space(2.0);

        egui::ScrollArea::vertical()
            .id_salt("diff_result_scroll")
            .max_height(250.0)
            .show(ui, |ui| {
                for dl in &self.diff_result {
                    let (prefix, color) = match dl.kind {
                        DiffKind::Equal => ("  ", egui::Color32::GRAY),
                        DiffKind::Added => ("+ ", egui::Color32::from_rgb(0, 180, 0)),
                        DiffKind::Removed => ("- ", egui::Color32::from_rgb(200, 0, 0)),
                    };
                    ui.label(egui::RichText::new(format!("{}{}", prefix, dl.line)).color(color).monospace());
                }
            });

        if !self.diff_result.is_empty() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Copy Diff").clicked() {
                    ui.ctx().copy_text(self.format_diff());
                }
                if ui.button("Save As...").clicked() {
                    let text = self.format_diff();
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "diff_result.txt") {
                        let _ = std::fs::write(path, text);
                    }
                }
            });
        }
    }
}

impl TextComparer {
    fn format_diff(&self) -> String {
        self.diff_result.iter().map(|dl| {
            let prefix = match dl.kind {
                DiffKind::Equal => "  ",
                DiffKind::Added => "+ ",
                DiffKind::Removed => "- ",
            };
            format!("{}{}", prefix, dl.line)
        }).collect::<Vec<_>>().join("\n")
    }
}

fn compute_diff(a: &str, b: &str) -> Vec<DiffLine> {
    let lines_a: Vec<&str> = a.lines().collect();
    let lines_b: Vec<&str> = b.lines().collect();
    let mut result = Vec::new();

    let lcs = lcs_indices(&lines_a, &lines_b);

    let mut a_idx = 0;
    let mut b_idx = 0;
    let mut lcs_idx = 0;

    while a_idx < lines_a.len() || b_idx < lines_b.len() {
        if lcs_idx < lcs.len() && a_idx == lcs[lcs_idx].0 && b_idx == lcs[lcs_idx].1 {
            result.push(DiffLine { line: lines_a[a_idx].to_string(), kind: DiffKind::Equal });
            a_idx += 1;
            b_idx += 1;
            lcs_idx += 1;
        } else if a_idx < lines_a.len() && (lcs_idx >= lcs.len() || a_idx < lcs[lcs_idx].0) {
            result.push(DiffLine { line: lines_a[a_idx].to_string(), kind: DiffKind::Removed });
            a_idx += 1;
        } else if b_idx < lines_b.len() {
            result.push(DiffLine { line: lines_b[b_idx].to_string(), kind: DiffKind::Added });
            b_idx += 1;
        }
    }

    result
}

fn lcs_indices(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}
