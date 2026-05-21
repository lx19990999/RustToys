use eframe::egui;
use crate::tool::{Tool, ToolCategory};
use crate::tr;
use crate::tools::async_utils::Pending;
use rand::Rng;

pub struct PasswordGenerator {
    output: String,
    length: usize,
    use_upper: bool,
    use_lower: bool,
    use_digits: bool,
    use_symbols: bool,
    exclude_chars: String,
    count: usize,
    save_result: String,
    pending_file: Pending<String>,
}

impl Default for PasswordGenerator {
    fn default() -> Self {
        Self {
            output: String::new(),
            length: 0,
            use_upper: false,
            use_lower: false,
            use_digits: false,
            use_symbols: false,
            exclude_chars: String::new(),
            count: 0,
            save_result: String::new(),
            pending_file: Pending::default(),
        }
    }
}

impl PasswordGenerator {
    fn init(&mut self) {
        if self.length == 0 {
            self.length = 16;
            self.use_upper = true;
            self.use_lower = true;
            self.use_digits = true;
            self.use_symbols = true;
            self.count = 5;
        }
    }

    fn do_generate(&mut self) {
        self.save_result.clear();
        let mut charset = String::new();
        if self.use_upper { charset.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ"); }
        if self.use_lower { charset.push_str("abcdefghijklmnopqrstuvwxyz"); }
        if self.use_digits { charset.push_str("0123456789"); }
        if self.use_symbols { charset.push_str("!@#$%^&*()-_=+[]{}|;:,.<>?"); }

        for ch in self.exclude_chars.chars() {
            charset = charset.replace(ch, "");
        }

        if charset.is_empty() {
            self.output = tr!("pw_no_options");
            return;
        }

        let chars: Vec<char> = charset.chars().collect();
        let mut rng = rand::thread_rng();
        let mut passwords = Vec::new();
        for _ in 0..self.count {
            let pw: String = (0..self.length)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect();
            passwords.push(pw);
        }
        self.output = passwords.join("\n");
    }
}

impl Tool for PasswordGenerator {
    fn name(&self) -> String { tr!("pw_name") }
    fn description(&self) -> String { tr!("pw_desc") }
    fn category(&self) -> ToolCategory { ToolCategory::Generators }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(text) = self.pending_file.poll() {
            self.save_result = text;
        }
        self.init();

        ui.horizontal(|ui| {
            ui.label(tr!("pw_length"));
            ui.add(egui::DragValue::new(&mut self.length).range(4..=128).speed(1));
            ui.separator();
            ui.label(tr!("label_count"));
            ui.add(egui::DragValue::new(&mut self.count).range(1..=50).speed(1));
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_upper, "A-Z");
            ui.checkbox(&mut self.use_lower, "a-z");
            ui.checkbox(&mut self.use_digits, "0-9");
            ui.checkbox(&mut self.use_symbols, "!@#$%");
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(tr!("pw_exclude"));
            ui.text_edit_singleline(&mut self.exclude_chars);
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let lbl_generate = tr!("btn_generate");
            if ui.button(lbl_generate).clicked() {
                self.do_generate();
            }
            let lbl_refresh = tr!("btn_refresh");
            if ui.button(lbl_refresh).clicked() {
                self.do_generate();
            }
        });

        ui.add_space(4.0);
        ui.label(tr!("label_output"));
        ui.add(
            egui::TextEdit::multiline(&mut self.output)
                .desired_width(f32::INFINITY)
                .desired_rows(8),
        );
        if !self.output.is_empty() {
            ui.horizontal(|ui| {
                let lbl_copy = tr!("btn_copy");
                if ui.button(lbl_copy).clicked() {
                    ui.ctx().copy_text(self.output.clone());
                }
                let lbl_save_as = tr!("btn_save_as");
                if ui.button(lbl_save_as).clicked() {
                    let title = tr!("save_as_title");
                    let filter_text = tr!("save_filter_text");
                    let default_name = tr!("pw_save_default");
                    crate::tools::async_utils::save_file_async(&mut self.pending_file, &title, &filter_text, &["txt"], &default_name, self.output.clone());
                }
            });
        }
        if !self.save_result.is_empty() {
            ui.colored_label(egui::Color32::GREEN, &self.save_result);
        }
    }
}
