use std::collections::HashMap;

use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::{Pending, open_file_async};

pub struct MorseCode {
    input: String,
    output: String,
    error: String,
    encode_mode: bool,
    pending_file: Pending<String>,
    save_pending: Pending<String>,
}

impl Default for MorseCode {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            error: String::new(),
            encode_mode: true,
            pending_file: Pending::default(),
            save_pending: Pending::default(),
        }
    }
}

impl Tool for MorseCode {
    fn name(&self) -> String { tr!("morse_name") }
    fn description(&self) -> String { tr!("morse_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Encoders }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let prev_input = self.input.clone();
        let prev_mode = self.encode_mode;

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
        let label_encode = tr!("label_encode");
        let label_decode = tr!("label_decode");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.encode_mode, true, &label_encode);
            ui.radio_value(&mut self.encode_mode, false, &label_decode);
        });
        ui.add_space(4.0);

        ui.columns(2, |cols| {
            // Left: Input
            cols[0].vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_paste")).clicked() {
                        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                            Ok(text) => self.input = text,
                            Err(e) => self.error = tr!("err_clipboard", e),
                        }
                    }
                    if ui.button(tr!("btn_open_file")).clicked() {
                        open_file_async(&mut self.pending_file, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"]);
                    }
                    if ui.button(tr!("btn_clear")).clicked() {
                        self.input.clear();
                        self.output.clear();
                        self.error.clear();
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_input"));

                egui::ScrollArea::vertical()
                    .id_salt("morse_input_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });

            // Right: Output
            cols[1].vertical(|ui| {
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.error);
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button(tr!("btn_copy")).clicked() && !self.output.is_empty() {
                        ui.ctx().copy_text(self.output.clone());
                    }
                    if ui.button(tr!("btn_save_as")).clicked() && !self.output.is_empty() {
                        crate::tools::async_utils::save_file_async(&mut self.save_pending, &tr!("save_as_title"), &tr!("save_filter_text"), &["txt"], &tr!("morse_save_default"), self.output.clone());
                    }
                });
                ui.add_space(2.0);
                ui.label(tr!("label_output"));

                egui::ScrollArea::vertical()
                    .id_salt("morse_output_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.output)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            });
        });

        // Auto-convert
        if self.input != prev_input || self.encode_mode != prev_mode {
            self.convert();
        }
    }
}

// ── Standard Morse table ──────────────────────────────────────────────

const MORSE_TABLE: &[(&str, &str)] = &[
    ("A", ".-"),    ("B", "-..."),  ("C", "-.-."),  ("D", "-.."),
    ("E", "."),     ("F", "..-."),  ("G", "--."),   ("H", "...."),
    ("I", ".."),    ("J", ".---"),  ("K", "-.-"),   ("L", ".-.."),
    ("M", "--"),    ("N", "-."),    ("O", "---"),   ("P", ".--."),
    ("Q", "--.-"),  ("R", ".-."),   ("S", "..."),   ("T", "-"),
    ("U", "..-"),   ("V", "...-"),  ("W", ".--"),   ("X", "-..-"),
    ("Y", "-.--"),  ("Z", "--.."),
    ("0", "-----"), ("1", ".----"), ("2", "..---"), ("3", "...--"),
    ("4", "....-"), ("5", "....."), ("6", "-...."), ("7", "--..."),
    ("8", "---.."), ("9", "----."),
    (".", ".-.-.-"), (",", "--..--"), ("?", "..--.."), ("'", ".----."),
    ("!", "-.-.--"), ("/", "-..-."), ("(", "-.--."), (")", "-.--.-"),
    ("&", ".-..."),  ("=", "-...-"), ("+", ".-.-."), ("-", "-....-"),
    ("_", "..--.-"), ("\"", ".-..-."), (";", "-.-.-."), (":", "---..."),
    ("$", "...-..-"), ("@", ".--.-."),
];

fn char_to_morse(ch: &str) -> Option<&'static str> {
    MORSE_TABLE.iter().find(|(c, _)| *c == ch).map(|(_, m)| *m)
}

fn morse_to_char(code: &str) -> Option<&'static str> {
    MORSE_TABLE.iter().find(|(_, m)| *m == code).map(|(c, _)| *c)
}

// ── Telegraph code lookup tables (from assets/chinese_telegraph.txt) ─

fn init_telegraph_tables() -> (HashMap<char, String>, HashMap<String, char>) {
    let mut char_to_code = HashMap::new();
    let mut code_to_char = HashMap::new();
    for line in TELEGRAPH_RAW.lines() {
        let mut parts = line.split('\t');
        if let (Some(ch), Some(code)) = (parts.next(), parts.next()) {
            if let (Some(c), true) = (ch.chars().next(), code.len() == 4) {
                char_to_code.insert(c, code.to_string());
                code_to_char.entry(code.to_string()).or_insert(c);
            }
        }
    }
    (char_to_code, code_to_char)
}

static TELEGRAPH_TABLES: std::sync::LazyLock<(HashMap<char, String>, HashMap<String, char>)> =
    std::sync::LazyLock::new(init_telegraph_tables);

const TELEGRAPH_RAW: &str = include_str!("../../../assets/chinese_telegraph.txt");

fn char_to_telegraph(ch: char) -> Option<String> {
    TELEGRAPH_TABLES.0.get(&ch).cloned()
}

fn telegraph_to_char(code: &str) -> Option<char> {
    TELEGRAPH_TABLES.1.get(code).copied()
}

// ── Encode: auto-detect Chinese / ASCII ───────────────────────────────

fn encode_mixed(text: &str) -> String {
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        let mut parts = Vec::new();
        for ch in word.chars() {
            if ch.is_ascii() {
                if let Some(m) = char_to_morse(&ch.to_uppercase().to_string()) {
                    parts.push(m.to_string());
                }
            } else if let Some(code) = char_to_telegraph(ch) {
                for digit in code.chars() {
                    if let Some(m) = char_to_morse(&digit.to_string()) {
                        parts.push(m.to_string());
                    }
                }
            } else {
                parts.push("?".to_string());
            }
        }
        words.push(parts.join(" "));
    }
    words.join(" / ")
}

// ── Decode: auto-detect standard morse / telegraph code ───────────────

fn decode_mixed(morse: &str) -> Result<String, String> {
    let mut result = String::new();
    for word_part in morse.trim().split('/') {
        let codes: Vec<&str> = word_part.split_whitespace().collect();
        let mut i = 0;
        while i < codes.len() {
            // Try reading 4 consecutive digits as a telegraph code
            if i + 3 < codes.len() {
                let mut digits = String::new();
                let mut all_digits = true;
                for j in 0..4 {
                    match morse_to_char(codes[i + j]) {
                        Some(ch) if ch.len() == 1 && ch.chars().next().unwrap().is_ascii_digit() => {
                            digits.push_str(ch);
                        }
                        _ => { all_digits = false; break; }
                    }
                }
                if all_digits {
                    if let Some(ch) = telegraph_to_char(&digits) {
                        result.push(ch);
                    } else {
                        result.push_str(&format!("[{}]", digits));
                    }
                    i += 4;
                    continue;
                }
            }
            // Otherwise decode as standard morse
            match morse_to_char(codes[i]) {
                Some(ch) => result.push_str(ch),
                None => return Err(tr!("morse_invalid_char", codes[i])),
            }
            i += 1;
        }
        result.push(' ');
    }
    Ok(result.trim().to_string())
}

// ── Main convert ──────────────────────────────────────────────────────

impl MorseCode {
    fn convert(&mut self) {
        self.error.clear();
        if self.input.is_empty() {
            self.output.clear();
            return;
        }

        if self.encode_mode {
            self.output = encode_mixed(&self.input);
        } else {
            match decode_mixed(&self.input) {
                Ok(text) => self.output = text,
                Err(e) => {
                    self.output.clear();
                    self.error = e;
                }
            }
        }
    }
}
