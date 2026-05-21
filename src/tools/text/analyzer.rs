use eframe::egui;
use crate::tr;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};
use rand::Rng;
use std::collections::HashMap;

pub struct TextAnalyzer {
    input: String,
    original_input: Option<String>,
    pending_file: Pending<String>,
}

impl Default for TextAnalyzer {
    fn default() -> Self {
        Self {
            input: String::new(),
            original_input: None,
            pending_file: Pending::default(),
        }
    }
}

impl Tool for TextAnalyzer {
    fn name(&self) -> String { tr!("analyzer_name") }
    fn description(&self) -> String { tr!("analyzer_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Text }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let total = ui.available_rect_before_wrap();
        let err_reading = tr!("err_error_reading");
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with(&err_reading) {
                self.input = text;
            }
        }
        let pad = 4.0;
        let w = total.width();
        let left_w = w * 0.6;
        let right_w = w - left_w - pad;
        let h = total.height();

        let label_h = 18.0;
        let btn_h = 22.0;
        let space = 2.0;
        let top_h = label_h + space + btn_h * 2.0 + space * 3.0;

        // --- Left: Text Input ---
        let left_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(left_w, h),
        );
        let lbl_paste = tr!("btn_paste");
        let lbl_open = tr!("btn_open_file");
        let lbl_clear = tr!("btn_clear");
        let lbl_copy = tr!("btn_copy");
        let lbl_save_as = tr!("btn_save_as");
        let lbl_show_original = tr!("btn_show_original");
        let lbl_input = tr!("label_input");
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_input).strong());
            ui.add_space(space);
            ui.horizontal_wrapped(|ui| {
                if ui.button(&lbl_paste).clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => { self.input = text; self.original_input = None; }
                        Err(e) => { self.input = tr!("err_clipboard", e); }
                    }
                }
                if ui.button(&lbl_open).clicked() {
                    open_file_async(&mut self.pending_file, &tr!("btn_open_file"), &tr!("analyzer_text"), &["txt"]);
                }
                if ui.button(&lbl_clear).clicked() {
                    self.input.clear();
                    self.original_input = None;
                }
                if ui.button(&lbl_copy).clicked() && !self.input.is_empty() {
                    ui.ctx().copy_text(self.input.clone());
                }
                if ui.button(&lbl_save_as).clicked() && !self.input.is_empty() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog(&tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("default_output_txt")) {
                        let _ = std::fs::write(path, &self.input);
                    }
                }
                if self.original_input.is_some() {
                    if ui.button(&lbl_show_original).clicked() {
                        if let Some(orig) = self.original_input.take() {
                            self.input = orig;
                        }
                    }
                }
            });
            ui.add_space(space);
            let text_h = (h - top_h).max(40.0);
            ui.add_sized(
                egui::vec2(left_w, text_h),
                egui::TextEdit::multiline(&mut self.input)
                    .font(egui::TextStyle::Monospace)
                    .id_salt("analyzer_input"),
            );
        });

        // Compute stats
        let stats = compute_stats(&self.input);
        let sel_stats = compute_selection_stats(ui, "analyzer_input", &self.input);

        // --- Right column ---
        let right_origin = total.min + egui::vec2(left_w + pad, 0.0);

        // Stats section (top portion of right column)
        let stats_h = 340.0;
        let stats_rect = egui::Rect::from_min_size(
            right_origin,
            egui::vec2(right_w, stats_h),
        );
        let lbl_stats = tr!("analyzer_stats");
        let lbl_selection = tr!("analyzer_selection");
        let lbl_sel_len = tr!("analyzer_sel_len");
        let lbl_sel_start = tr!("analyzer_sel_start");
        let lbl_sel_end = tr!("analyzer_sel_end");
        let lbl_sel_line = tr!("analyzer_sel_line");
        let lbl_sel_col = tr!("analyzer_sel_col");
        let lbl_text = tr!("analyzer_text");
        let lbl_bytes = tr!("analyzer_bytes");
        let lbl_chars = tr!("analyzer_chars");
        let lbl_words = tr!("analyzer_words");
        let lbl_sentences = tr!("analyzer_sentences");
        let lbl_paragraphs = tr!("analyzer_paragraphs");
        let lbl_lines = tr!("analyzer_lines");
        let lbl_line_break = tr!("analyzer_line_break");
        let lbl_char_freq = tr!("analyzer_char_freq");
        let lbl_word_freq = tr!("analyzer_word_freq");
        ui.scope_builder(egui::UiBuilder::new().max_rect(stats_rect), |ui| {
            ui.label(egui::RichText::new(&lbl_stats).strong());
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("analyzer_stats_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Selection stats
                    ui.label(egui::RichText::new(&lbl_selection).underline().strong());
                    ui.add_space(2.0);
                    egui::Grid::new("sel_stats_grid")
                        .num_columns(2)
                        .spacing([12.0, 1.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(&lbl_sel_len); ui.label(format!("{}", sel_stats.len)); ui.end_row();
                            ui.label(&lbl_sel_start); ui.label(format!("{}", sel_stats.start)); ui.end_row();
                            ui.label(&lbl_sel_end); ui.label(format!("{}", sel_stats.end)); ui.end_row();
                            ui.label(&lbl_sel_line); ui.label(format!("{}", sel_stats.line)); ui.end_row();
                            ui.label(&lbl_sel_col); ui.label(format!("{}", sel_stats.col)); ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(&lbl_text).underline().strong());
                    ui.add_space(2.0);
                    egui::Grid::new("text_stats_grid")
                        .num_columns(2)
                        .spacing([12.0, 1.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(&lbl_bytes); ui.label(format!("{}", stats.bytes)); ui.end_row();
                            ui.label(&lbl_chars); ui.label(format!("{}", stats.chars)); ui.end_row();
                            ui.label(&lbl_words); ui.label(format!("{}", stats.words)); ui.end_row();
                            ui.label(&lbl_sentences); ui.label(format!("{}", stats.sentences)); ui.end_row();
                            ui.label(&lbl_paragraphs); ui.label(format!("{}", stats.paragraphs)); ui.end_row();
                            ui.label(&lbl_lines); ui.label(format!("{}", stats.lines)); ui.end_row();
                            ui.label(&lbl_line_break); ui.label(&stats.line_break_type); ui.end_row();
                        });

                    if !stats.char_freq.is_empty() || !stats.word_freq.is_empty() {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(&lbl_char_freq).underline().strong());
                        ui.add_space(2.0);
                        egui::Grid::new("char_freq_grid")
                            .num_columns(2)
                            .spacing([12.0, 1.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for (ch, count) in &stats.char_freq {
                                    let display = if *ch == ' ' { "\u{23B5}".to_string() } else { ch.to_string() };
                                    ui.label(egui::RichText::new(display).monospace());
                                    ui.label(format!("{}", count));
                                    ui.end_row();
                                }
                            });

                        if !stats.word_freq.is_empty() {
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(&lbl_word_freq).underline().strong());
                            ui.add_space(2.0);
                            egui::Grid::new("word_freq_grid")
                                .num_columns(2)
                                .spacing([12.0, 1.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for (word, count) in &stats.word_freq {
                                        ui.label(egui::RichText::new(word.as_str()).monospace());
                                        ui.label(format!("{}", count));
                                        ui.end_row();
                                    }
                                });
                        }
                    }
                });
        });

        // Actions section (bottom portion of right column)
        let actions_y = right_origin.y + stats_h + pad;
        let actions_h = (h - stats_h - pad).max(120.0);
        let actions_rect = egui::Rect::from_min_size(
            egui::pos2(right_origin.x, actions_y),
            egui::vec2(right_w, actions_h),
        );
        let lbl_convert_line_break = tr!("analyzer_convert_line_break");
        let lbl_lf = tr!("analyzer_lf");
        let lbl_crlf = tr!("analyzer_crlf");
        let lbl_convert_case = tr!("analyzer_convert_case");
        let lbl_sort_lines = tr!("analyzer_sort_lines");
        let lbl_alphabetize = tr!("analyzer_alphabetize");
        let lbl_reverse_alpha = tr!("analyzer_reverse_alpha");
        let lbl_by_last_word = tr!("analyzer_by_last_word");
        let lbl_reverse_by_last_word = tr!("analyzer_reverse_by_last_word");
        let lbl_reverse = tr!("analyzer_reverse");
        let lbl_randomize = tr!("analyzer_randomize");
        ui.scope_builder(egui::UiBuilder::new().max_rect(actions_rect), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("analyzer_actions_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Convert Line Break
                    ui.label(egui::RichText::new(&lbl_convert_line_break).strong());
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(&lbl_lf).clicked() {
                            self.save_original();
                            self.input = self.input.replace("\r\n", "\n").replace('\r', "\n");
                        }
                        if ui.button(&lbl_crlf).clicked() {
                            self.save_original();
                            self.input = self.input.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(&lbl_convert_case).strong());
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        let cases = [
                            ("lower case", CaseType::Lower),
                            ("UPPER CASE", CaseType::Upper),
                            ("Sentence case", CaseType::Sentence),
                            ("Title Case", CaseType::Title),
                            ("camelCase", CaseType::Camel),
                            ("PascalCase", CaseType::Pascal),
                            ("snake_case", CaseType::Snake),
                            ("CONSTANT_CASE", CaseType::Constant),
                            ("kebab-case", CaseType::Kebab),
                            ("COBOL-CASE", CaseType::Cobol),
                            ("Train-Case", CaseType::Train),
                            ("aLtErNaTiNg", CaseType::Alternating),
                            ("InVeRsE", CaseType::Inverse),
                            ("raNdoM", CaseType::Random),
                        ];
                        for (label, case_type) in &cases {
                            if ui.button(*label).clicked() {
                                self.save_original();
                                self.input = convert_case(&self.input, *case_type);
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(&lbl_sort_lines).strong());
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(&lbl_alphabetize).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            lines.sort();
                            self.input = lines.join("\n");
                        }
                        if ui.button(&lbl_reverse_alpha).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            lines.sort();
                            lines.reverse();
                            self.input = lines.join("\n");
                        }
                        if ui.button(&lbl_by_last_word).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            lines.sort_by_key(|l| l.split_whitespace().last().unwrap_or("").to_string());
                            self.input = lines.join("\n");
                        }
                        if ui.button(&lbl_reverse_by_last_word).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            lines.sort_by_key(|l| l.split_whitespace().last().unwrap_or("").to_string());
                            lines.reverse();
                            self.input = lines.join("\n");
                        }
                        if ui.button(&lbl_reverse).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            lines.reverse();
                            self.input = lines.join("\n");
                        }
                        if ui.button(&lbl_randomize).clicked() {
                            self.save_original();
                            let mut lines: Vec<&str> = self.input.lines().collect();
                            let mut rng = rand::thread_rng();
                            for i in (1..lines.len()).rev() {
                                let j = rng.gen_range(0..=i);
                                lines.swap(i, j);
                            }
                            self.input = lines.join("\n");
                        }
                    });
                });
        });
    }
}

impl TextAnalyzer {
    fn save_original(&mut self) {
        if self.original_input.is_none() {
            self.original_input = Some(self.input.clone());
        }
    }
}

// --- Case conversion ---

#[derive(Clone, Copy)]
enum CaseType {
    Lower,
    Upper,
    Sentence,
    Title,
    Camel,
    Pascal,
    Snake,
    Constant,
    Kebab,
    Cobol,
    Train,
    Alternating,
    Inverse,
    Random,
}

fn split_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_upper = false;

    for ch in s.chars() {
        if ch.is_alphanumeric() {
            let is_upper = ch.is_uppercase();
            if !current.is_empty() && is_upper && !prev_upper {
                // camelCase boundary: aB -> a, B
                words.push(std::mem::take(&mut current));
            } else if !current.is_empty() && !is_upper && prev_upper {
                // ABCd -> AB, Cd
                if current.len() > 1 {
                    let last = current.pop().unwrap();
                    words.push(std::mem::take(&mut current));
                    current.push(last);
                }
            }
            current.push(ch.to_ascii_lowercase());
            prev_upper = is_upper;
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
            prev_upper = false;
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn convert_case(text: &str, case_type: CaseType) -> String {
    match case_type {
        CaseType::Lower => text.to_lowercase(),
        CaseType::Upper => text.to_uppercase(),
        CaseType::Sentence => {
            let mut result = String::new();
            let mut capitalize_next = true;
            for ch in text.chars() {
                if capitalize_next && ch.is_alphabetic() {
                    for uc in ch.to_uppercase() { result.push(uc); }
                    capitalize_next = false;
                } else {
                    result.push(ch.to_ascii_lowercase());
                }
                if ch == '.' || ch == '!' || ch == '?' { capitalize_next = true; }
            }
            result
        }
        CaseType::Title => {
            text.lines().map(|line| {
                let words: Vec<String> = line.split_whitespace()
                    .map(|w| capitalize_first(w))
                    .collect();
                words.join(" ")
            }).collect::<Vec<_>>().join("\n")
        }
        CaseType::Camel => {
            let words = split_words(text);
            words.into_iter().enumerate().map(|(i, w)| {
                if i == 0 { w } else { capitalize_first(&w) }
            }).collect()
        }
        CaseType::Pascal => {
            let words = split_words(text);
            words.into_iter().map(|w| capitalize_first(&w)).collect()
        }
        CaseType::Snake => {
            let words = split_words(text);
            words.join("_")
        }
        CaseType::Constant => {
            let words = split_words(text);
            words.join("_").to_uppercase()
        }
        CaseType::Kebab => {
            let words = split_words(text);
            words.join("-")
        }
        CaseType::Cobol => {
            let words = split_words(text);
            words.join("-").to_uppercase()
        }
        CaseType::Train => {
            let words = split_words(text);
            words.iter().map(|w| capitalize_first(w)).collect::<Vec<_>>().join("-")
        }
        CaseType::Alternating => {
            let mut idx = 0u32;
            text.chars().map(|ch| {
                if ch.is_alphabetic() {
                    let r = if idx % 2 == 0 { ch.to_ascii_lowercase() } else { ch.to_ascii_uppercase() };
                    idx += 1;
                    r
                } else { ch }
            }).collect()
        }
        CaseType::Inverse => {
            let mut idx = 0u32;
            text.chars().map(|ch| {
                if ch.is_alphabetic() {
                    let r = if idx % 2 == 0 { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() };
                    idx += 1;
                    r
                } else { ch }
            }).collect()
        }
        CaseType::Random => {
            let mut rng = rand::thread_rng();
            text.chars().map(|ch| {
                if ch.is_alphabetic() {
                    if rng.gen_bool(0.5) { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() }
                } else { ch }
            }).collect()
        }
    }
}

// --- Statistics ---

struct TextStats {
    bytes: usize,
    chars: usize,
    words: usize,
    sentences: usize,
    paragraphs: usize,
    lines: usize,
    line_break_type: String,
    char_freq: Vec<(char, usize)>,
    word_freq: Vec<(String, usize)>,
}

struct SelectionStats {
    len: usize,
    start: usize,
    end: usize,
    line: usize,
    col: usize,
}

fn compute_stats(text: &str) -> TextStats {
    let bytes = text.len();
    let chars = text.chars().count();
    let words_vec: Vec<&str> = text.split_whitespace().collect();
    let words = words_vec.len();
    let sentences = text.split(|c: char| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .count();
    let paragraphs = if text.trim().is_empty() { 0 } else {
        text.split("\n\n").filter(|s| !s.trim().is_empty()).count()
    };
    let lines = if text.is_empty() { 0 } else { text.lines().count() };

    // Line break detection
    let has_crlf = text.contains("\r\n");
    let has_lf = text.contains('\n') && !text.contains("\r\n");
    let has_cr = text.contains('\r') && !text.contains("\r\n");
    let line_break_type = match (has_crlf, has_lf, has_cr) {
        (true, false, false) => "CRLF",
        (false, true, false) => "LF",
        (false, false, true) => "CR",
        (true, true, _) | (true, _, true) => "Mixed",
        _ => {
            if text.is_empty() { "Unknown" } else { "LF" }
        }
    }.to_string();

    // Character frequency
    let mut char_map: HashMap<char, usize> = HashMap::new();
    for ch in text.chars() {
        *char_map.entry(ch).or_insert(0) += 1;
    }
    let mut char_freq: Vec<(char, usize)> = char_map.into_iter().collect();
    char_freq.sort_by(|a, b| b.1.cmp(&a.1));

    // Word frequency (words >= 2 chars)
    let mut word_map: HashMap<String, usize> = HashMap::new();
    for w in &words_vec {
        if w.len() >= 2 {
            *word_map.entry(w.to_lowercase()).or_insert(0) += 1;
        }
    }
    let mut word_freq: Vec<(String, usize)> = word_map.into_iter().collect();
    word_freq.sort_by(|a, b| b.1.cmp(&a.1));
    word_freq.truncate(20);

    TextStats { bytes, chars, words, sentences, paragraphs, lines, line_break_type, char_freq, word_freq }
}

fn compute_selection_stats(ui: &egui::Ui, id_salt: &str, text: &str) -> SelectionStats {
    let id = ui.id().with(id_salt);
    let state = egui::TextEdit::load_state(ui.ctx(), id);

    if let Some(state) = state {
        let cursor = state.cursor;
        let range_opt = cursor.char_range();
        if let Some(sel_range) = range_opt {
            let primary = sel_range.primary.index;
            let secondary = sel_range.secondary.index;
            let start = primary.min(secondary);
            let end = primary.max(secondary);
            let sel_len = end - start;

            // Line and column from start position
            let before = &text[..start.min(text.len())];
            let line = before.lines().count();
            let col = before.lines().last().map(|l| l.len() + 1).unwrap_or(1);

            return SelectionStats { len: sel_len, start, end, line, col };
        }
    }

    // No selection — report cursor at end
    let len = text.len();
    let lines = if text.is_empty() { 0 } else { text.lines().count() };
    let col = text.lines().last().map(|l| l.len() + 1).unwrap_or(1);
    SelectionStats { len: 0, start: len, end: len, line: lines + 1, col }
}
