use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tools::async_utils::{Pending, open_file_async};

pub struct RegexTester {
    pattern: String,
    test_string: String,
    matches: String,
    error: String,
    prev_pattern: String,
    prev_test: String,
    pending_file: Pending<String>,
}

impl Default for RegexTester {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            test_string: String::new(),
            matches: String::new(),
            error: String::new(),
            prev_pattern: String::new(),
            prev_test: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl RegexTester {
    fn do_match(&mut self) {
        self.error.clear();
        self.matches.clear();

        if self.pattern.trim().is_empty() || self.test_string.trim().is_empty() {
            return;
        }

        match regex::Regex::new(&self.pattern) {
            Ok(re) => {
                let mut results = Vec::new();
                for (i, cap) in re.captures_iter(&self.test_string).enumerate() {
                    let full_match = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                    let start = cap.get(0).map(|m| m.start()).unwrap_or(0);
                    let end = cap.get(0).map(|m| m.end()).unwrap_or(0);
                    let mut line = format!("Match {}: '{}' (pos {}-{})", i + 1, full_match, start, end);
                    for (j, group) in cap.iter().enumerate().skip(1) {
                        if let Some(m) = group {
                            line.push_str(&format!("\n  Group {}: '{}'", j, m.as_str()));
                        }
                    }
                    results.push(line);
                }

                if results.is_empty() {
                    self.matches = "No match found".to_string();
                } else {
                    let count = re.find_iter(&self.test_string).count();
                    self.matches = format!("Total matches: {}\n\n{}", count, results.join("\n\n"));
                }
            }
            Err(e) => self.error = format!("Invalid regex: {}", e),
        }
    }

    fn auto_match(&mut self) {
        if self.pattern != self.prev_pattern || self.test_string != self.prev_test {
            self.prev_pattern = self.pattern.clone();
            self.prev_test = self.test_string.clone();
            self.do_match();
        }
    }
}

impl Tool for RegexTester {
    fn name(&self) -> &str { "Regular Expression Tester" }
    fn description(&self) -> &str { "Test regular expressions and see matches" }
    fn category(&self) -> ToolCategory { ToolCategory::Testers }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            if !text.starts_with("Error reading file:") {
                self.test_string = text;
                self.error.clear();
            }
        }

        let total = ui.available_rect_before_wrap();
        let pad = 4.0;
        let w = total.width();
        let half_w = (w - pad) * 0.5;

        // Layout constants
        let label_h = 18.0;
        let btn_h = 22.0;
        let space = 2.0;
        let query_h = 24.0;
        let error_h = if self.error.is_empty() { 0.0 } else { 16.0 };
        let top_header_h = label_h + space + btn_h + space + space;
        let cheat_header_h = 20.0 + space;

        let cols_h = (total.height() * 0.55).max(120.0);
        let cheat_h = (total.height() - cols_h - query_h - error_h - cheat_header_h - pad * 3.0).max(60.0);

        // --- Left column: Test String ---
        let left_rect = egui::Rect::from_min_size(
            total.min,
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.label(egui::RichText::new("Text").strong());
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Paste").clicked() {
                    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                        Ok(text) => { self.test_string = text; self.error.clear(); }
                        Err(e) => self.error = format!("Clipboard error: {}", e),
                    }
                }
                if ui.button("Open File...").clicked() {
                    open_file_async(&mut self.pending_file, "Open text file", "Text", &["txt"]);
                }
                if ui.button("Clear").clicked() {
                    self.test_string.clear();
                    self.matches.clear();
                    self.error.clear();
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.test_string)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Right column: Match information ---
        let right_rect = egui::Rect::from_min_size(
            total.min + egui::vec2(half_w + pad, 0.0),
            egui::vec2(half_w, cols_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.label(egui::RichText::new("Match information").strong());
            ui.add_space(space);
            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() && !self.matches.is_empty() {
                    ui.ctx().copy_text(self.matches.clone());
                }
                if ui.button("Save As...").clicked() && !self.matches.is_empty() {
                    if let Some(path) = crate::tools::async_utils::save_file_dialog("Save as", "Text", &["txt"], "regex_matches.txt") {
                        let _ = std::fs::write(path, &self.matches);
                    }
                }
            });
            ui.add_space(space);
            let text_h = (cols_h - top_header_h).max(40.0);
            ui.add_sized(
                egui::vec2(half_w, text_h),
                egui::TextEdit::multiline(&mut self.matches)
                    .font(egui::TextStyle::Monospace),
            );
        });

        // --- Regex input ---
        let query_y = total.min.y + cols_h + pad;
        let query_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, query_y),
            egui::vec2(w, query_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(query_rect), |ui| {
            ui.horizontal(|ui| {
                ui.label("Regular expression:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.pattern)
                        .desired_width(ui.available_width())
                        .hint_text(r"Enter regex, e.g. \d+"),
                );
            });
        });

        if !self.error.is_empty() {
            let error_y = query_y + query_h;
            let error_rect = egui::Rect::from_min_size(
                egui::pos2(total.min.x, error_y),
                egui::vec2(w, error_h),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(error_rect), |ui| {
                ui.colored_label(egui::Color32::RED, &self.error);
            });
        }

        // Auto-match on input change
        self.auto_match();

        // --- Cheatsheet fills remaining height ---
        let cheat_y = query_y + query_h + error_h + pad;
        let cheat_rect = egui::Rect::from_min_size(
            egui::pos2(total.min.x, cheat_y),
            egui::vec2(w, cheat_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(cheat_rect), |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Cheat sheet").strong());
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("rx_cheatsheet_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Character classes
                        ui.label(egui::RichText::new("Character classes").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_chars", &[
                            (".", "Any character except newline"),
                            ("[abc]", "A single character of a, b or c"),
                            ("[^abc]", "A character except a, b or c"),
                            ("[a-z]", "Character range"),
                            ("[\\s\\S]", "Match any"),
                            ("\\w", "Word"),
                            ("\\W", "Not word"),
                            ("\\d", "Digit"),
                            ("\\D", "Not digit"),
                            ("\\s", "Whitespace"),
                            ("\\S", "Not whitespace"),
                            ("\\h", "Horizontal whitespace"),
                            ("\\H", "Not horizontal whitespace"),
                            ("\\v", "Vertical whitespace"),
                            ("\\V", "Not vertical whitespace"),
                            ("\\R", "Line break"),
                            ("\\N", "Not line break"),
                            ("\\p{L}", "Unicode category"),
                            ("\\P{L}", "Not unicode category"),
                            ("\\p{Han}", "Unicode script"),
                            ("\\P{Han}", "Not unicode script"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Anchors").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_anchors", &[
                            ("\\A", "Beginning of string"),
                            ("\\Z", "End of string"),
                            ("\\z", "Strict end of string"),
                            ("^", "Beginning"),
                            ("$", "End"),
                            ("\\b", "Word boundary"),
                            ("\\B", "Not word boundary"),
                            ("\\G", "Previous match end"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Escaped characters").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_escaped", &[
                            ("\\+", "Reserved characters"),
                            ("\\000", "Octal escape"),
                            ("\\xFF", "Hexadecimal escape"),
                            ("\\x{FF}", "Unicode escape"),
                            ("\\cI", "Control character escape"),
                            ("\\Q...\\E", "Escape sequence"),
                            ("\\t", "Tab"),
                            ("\\n", "Line feed"),
                            ("\\f", "Form feed"),
                            ("\\r", "Carriage return"),
                            ("\\0", "Null"),
                            ("\\a", "Bell"),
                            ("\\e", "Esc"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Groups and References").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_groups", &[
                            ("(abc)", "Capturing group"),
                            ("(?<name>abc)", "Named capturing group"),
                            ("\\k'name'", "Named reference"),
                            ("\\1", "Numeric reference"),
                            ("(?:abc)", "Non-capturing group"),
                            ("(?>abc)", "Atomic group"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Lookaround").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_lookaround", &[
                            ("(?=abc)", "Positive lookahead"),
                            ("(?!abc)", "Negative lookahead"),
                            ("(?<=abc)", "Positive lookbehind"),
                            ("(?<!abc)", "Negative lookbehind"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Quantifiers and Alternation").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_quantifiers", &[
                            ("+", "Plus"),
                            ("*", "Star"),
                            ("{1,3}", "Quantifier"),
                            ("?", "Optional"),
                            ("?", "Lazy"),
                            ("+", "Possessive"),
                            ("|", "Alternation"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Special").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_special", &[
                            ("(?#foo)", "Comment"),
                            ("(?(?=a)b|c)", "Conditional"),
                            ("(?(1)b|c)", "Group conditional"),
                            ("(?R)", "Recursion"),
                            ("(?i)", "Mode modifier"),
                        ], &mut self.pattern);

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Substitution").underline().strong());
                        ui.add_space(2.0);
                        render_cheatsheet(ui, "rx_substitution", &[
                            ("$0", "Match"),
                            ("$1", "Capture group"),
                            ("\\n", "Escaped characters"),
                        ], &mut self.pattern);
                    });
            });
        });
    }
}

fn render_cheatsheet(ui: &mut egui::Ui, id_salt: &str, entries: &[(&str, &str)], pattern: &mut String) {
    egui::Grid::new(id_salt)
        .num_columns(2)
        .spacing([16.0, 2.0])
        .striped(true)
        .show(ui, |ui| {
            for (syntax, desc) in entries {
                if ui.link(egui::RichText::new(*syntax).monospace().color(egui::Color32::from_rgb(0, 120, 200))).clicked() {
                    *pattern = syntax.to_string();
                }
                ui.label(*desc);
                ui.end_row();
            }
        });
}
